"""
keryx-dewatermark — Suppression de watermark NotebookLM sur images et vidéos.

Deux modes :
  - CV2 pur (CPU, rapide)  : inpainting Telea — utilisé par défaut
  - SDXL (GPU, optionnel)  : activé si USE_SDXL=true et GPU disponible

POST /clean/image  — supprime le watermark d'une image
POST /clean/video  — supprime le watermark de tous les frames d'une vidéo
"""
import os
import io
import uuid
import time
import asyncio
import logging
import tempfile
import shutil
import subprocess
import pathlib
from typing import Optional

import cv2
import numpy as np
from PIL import Image
import aioboto3
from fastapi import FastAPI, HTTPException
from pydantic import BaseModel
from urllib.parse import urlparse
import httpx

logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s [%(levelname)s] %(name)s: %(message)s",
    datefmt="%Y-%m-%d %H:%M:%S",
)
logger = logging.getLogger("keryx.dewatermark")

class HealthCheckFilter(logging.Filter):
    def filter(self, record: logging.LogRecord) -> bool:
        if "/health" in record.getMessage():
            record.levelno = logging.DEBUG
            record.levelname = "DEBUG"
        return True

app = FastAPI(title="Keryx Dewatermark", version="1.0.0")

SERVICE_NAME = "keryx-dewatermark"
S3_ENDPOINT = os.getenv("S3_ENDPOINT", "https://minio-170-api.zacharie.org")
S3_ACCESS_KEY = os.getenv("S3_ACCESS_KEY_ID") or os.getenv("AWS_ACCESS_KEY_ID")
S3_SECRET_KEY = os.getenv("S3_SECRET_ACCESS_KEY") or os.getenv("AWS_SECRET_ACCESS_KEY")
S3_BUCKET = os.getenv("S3_BUCKET", "keryx")
USE_SDXL = os.getenv("USE_SDXL", "false").lower() == "true"

# ── SDXL optionnel ──────────────────────────────────────────────────────────
sdxl_pipe = None
if USE_SDXL:
    try:
        import torch
        from diffusers import StableDiffusionXLInpaintPipeline
        DEVICE = "cuda" if torch.cuda.is_available() else "cpu"
        logger.info(f"Loading SDXL Inpaint pipeline on {DEVICE}...")
        sdxl_pipe = StableDiffusionXLInpaintPipeline.from_pretrained(
            "stabilityai/stable-diffusion-xl-base-1.0",
            torch_dtype=torch.float16 if DEVICE == "cuda" else torch.float32,
            use_safetensors=True,
        )
        if DEVICE == "cuda":
            sdxl_pipe.enable_model_cpu_offload()
            sdxl_pipe.enable_attention_slicing()
        else:
            sdxl_pipe.to(DEVICE)
        logger.info("SDXL pipeline loaded.")
    except Exception as e:
        logger.warning(f"Could not load SDXL: {e}. Falling back to CV2 only.")
        sdxl_pipe = None

s3_session = aioboto3.Session()


def _s3_client():
    return s3_session.client(
        "s3",
        endpoint_url=S3_ENDPOINT,
        aws_access_key_id=S3_ACCESS_KEY,
        aws_secret_access_key=S3_SECRET_KEY,
        verify=False,
    )


# ── CV2 Watermark Removal ───────────────────────────────────────────────────

def remove_notebooklm_watermark_cv2(image: Image.Image) -> Image.Image:
    """
    Suppression par MedianBlur difference + inpainting Telea (CV2 pur, CPU).
    Cible le coin bas-droit (10% hauteur × 25% largeur).
    """
    img_bgr = cv2.cvtColor(np.array(image.convert("RGB")), cv2.COLOR_RGB2BGR)
    h, w = img_bgr.shape[:2]

    # ROI : bas-droit
    mx, my = int(w * 0.25), int(h * 0.10)
    y0, x0 = h - my, w - mx
    roi = img_bgr[y0:h, x0:w].copy()

    rh, rw = roi.shape[:2]
    if rh < 5 or rw < 5:
        return image

    # Masque par différence médiane
    ksize = max(11, min(31, (min(rh, rw) // 6) | 1))
    background = cv2.medianBlur(roi, ksize)
    diff_gray = cv2.cvtColor(cv2.absdiff(roi, background), cv2.COLOR_BGR2GRAY)
    _, binary = cv2.threshold(diff_gray, 30, 255, cv2.THRESH_BINARY)

    num_labels, labels, stats, _ = cv2.connectedComponentsWithStats(binary, connectivity=8)
    mask = np.zeros((rh, rw), dtype=np.uint8)
    found = False
    for i in range(1, num_labels):
        area = stats[i, cv2.CC_STAT_AREA]
        if 100 < area < rh * rw * 0.5:
            mask[labels == i] = 255
            found = True

    if not found:
        return image

    kernel = cv2.getStructuringElement(cv2.MORPH_ELLIPSE, (3, 3))
    mask = cv2.dilate(mask, kernel, iterations=2)

    cleaned_roi = cv2.inpaint(roi, mask, 3, cv2.INPAINT_TELEA)
    img_bgr[y0:h, x0:w] = cleaned_roi

    return Image.fromarray(cv2.cvtColor(img_bgr, cv2.COLOR_BGR2RGB))


def remove_watermark_sdxl(image: Image.Image) -> Image.Image:
    """Inpainting SDXL si GPU disponible."""
    if sdxl_pipe is None:
        return remove_notebooklm_watermark_cv2(image)

    import torch
    img = image.resize((1024, 1024))
    h_orig, w_orig = image.size[1], image.size[0]

    # Construire le masque (bas-droit 10%×25%)
    mask = Image.new("L", (1024, 1024), 0)
    import PIL.ImageDraw as ImageDraw
    draw = ImageDraw.Draw(mask)
    draw.rectangle([768, 896, 1024, 1024], fill=255)

    with torch.inference_mode():
        result = sdxl_pipe(
            prompt="clean slide background, professional presentation, no watermark",
            negative_prompt="watermark, logo, text overlay",
            image=img,
            mask_image=mask,
            num_inference_steps=20,
            strength=0.85,
        ).images[0]

    return result.resize((w_orig, h_orig))


# ── S3 helpers ───────────────────────────────────────────────────────────────

async def download_image_s3(url: str) -> Image.Image:
    parsed = urlparse(url)
    if url.startswith("/") and os.path.exists(url):
        return Image.open(url).convert("RGB")
    if any(h in parsed.netloc for h in ["zacharie.org", "minio", "localhost", "rustfs"]):
        parts = parsed.path.lstrip("/").split("/")
        bucket, key = parts[0], "/".join(parts[1:])
        async with _s3_client() as s3:
            resp = await s3.get_object(Bucket=bucket, Key=key)
            data = await resp["Body"].read()
        return Image.open(io.BytesIO(data)).convert("RGB")
    async with httpx.AsyncClient(verify=False) as client:
        resp = await client.get(url)
        return Image.open(io.BytesIO(resp.content)).convert("RGB")


async def upload_image_s3(image: Image.Image, key: str) -> str:
    buf = io.BytesIO()
    image.save(buf, format="PNG")
    buf.seek(0)
    async with _s3_client() as s3:
        await s3.put_object(Bucket=S3_BUCKET, Key=key, Body=buf.read(), ContentType="image/png")
    return f"{S3_ENDPOINT}/{S3_BUCKET}/{key}"


async def download_file_s3(url: str, dest: str):
    parsed = urlparse(url)
    if url.startswith("/") and os.path.exists(url):
        shutil.copy(url, dest)
        return
    if any(h in parsed.netloc for h in ["zacharie.org", "minio", "localhost", "rustfs"]):
        parts = parsed.path.lstrip("/").split("/")
        bucket, key = parts[0], "/".join(parts[1:])
        async with _s3_client() as s3:
            await s3.download_file(bucket, key, dest)
    else:
        async with httpx.AsyncClient(verify=False) as client:
            async with client.stream("GET", url) as resp:
                with open(dest, "wb") as f:
                    async for chunk in resp.aiter_bytes(8192):
                        f.write(chunk)


async def upload_video_s3(path: str, key: str) -> str:
    async with _s3_client() as s3:
        await s3.upload_file(path, S3_BUCKET, key, ExtraArgs={"ContentType": "video/mp4"})
    return f"{S3_ENDPOINT}/{S3_BUCKET}/{key}"


# ── API Models ───────────────────────────────────────────────────────────────

class ImageCleanRequest(BaseModel):
    image_url: str
    job_id: str
    use_sdxl: bool = False  # Force SDXL même si non activé globalement
    output_key: Optional[str] = None


class VideoCleanRequest(BaseModel):
    video_url: str
    job_id: str
    fps_override: Optional[float] = None
    output_key: Optional[str] = None


# ── Endpoints ────────────────────────────────────────────────────────────────

@app.get("/health")
def health():
    return {
        "status": "ok",
        "service": SERVICE_NAME,
        "version": "1.0.0",
        "sdxl_enabled": sdxl_pipe is not None,
    }


@app.post("/clean/image")
async def clean_image(req: ImageCleanRequest):
    request_id = str(uuid.uuid4())[:8]
    logger.info(f"[{request_id}] clean/image job={req.job_id} url={req.image_url}")
    start_time = time.time()
    try:
        image = await download_image_s3(req.image_url)

        if req.use_sdxl and sdxl_pipe is not None:
            cleaned = await asyncio.to_thread(remove_watermark_sdxl, image)
        else:
            cleaned = await asyncio.to_thread(remove_notebooklm_watermark_cv2, image)

        key = req.output_key or f"{req.job_id}/dewatermark/{uuid.uuid4()}.png"
        result_url = await upload_image_s3(cleaned, key)

        elapsed = time.time() - start_time
        logger.info(f"[{request_id}] Done in {elapsed:.1f}s → {result_url}")
        return {"status": "success", "url": result_url, "duration": f"{elapsed:.2f}s"}

    except Exception as e:
        logger.exception(f"[{request_id}] Error")
        raise HTTPException(500, str(e))


@app.post("/clean/video")
async def clean_video(req: VideoCleanRequest):
    request_id = str(uuid.uuid4())[:8]
    logger.info(f"[{request_id}] clean/video job={req.job_id} url={req.video_url}")
    start_time = time.time()

    tmp_dir = tempfile.mkdtemp(prefix=f"keryx_dw_{request_id}_")
    try:
        # 1. Download
        video_path = os.path.join(tmp_dir, "input.mp4")
        logger.info(f"[{request_id}] Downloading video...")
        await download_file_s3(req.video_url, video_path)

        if not os.path.exists(video_path) or os.path.getsize(video_path) == 0:
            raise HTTPException(500, "Video download failed or empty")

        # 2. Extract frames with ffmpeg
        raw_dir = os.path.join(tmp_dir, "raw")
        clean_dir = os.path.join(tmp_dir, "clean")
        os.makedirs(raw_dir, exist_ok=True)
        os.makedirs(clean_dir, exist_ok=True)

        # Get FPS
        probe = subprocess.run(
            ["ffprobe", "-v", "error", "-select_streams", "v:0",
             "-show_entries", "stream=r_frame_rate", "-of", "csv=p=0", video_path],
            capture_output=True, text=True
        )
        fps = req.fps_override or 24.0
        if probe.returncode == 0 and probe.stdout.strip():
            try:
                num, den = probe.stdout.strip().split("/")
                fps = float(num) / float(den)
            except Exception:
                pass

        logger.info(f"[{request_id}] Extracting frames at {fps:.2f} fps...")
        subprocess.run(
            ["ffmpeg", "-y", "-i", video_path, "-q:v", "2",
             os.path.join(raw_dir, "frame_%04d.jpg")],
            check=True, capture_output=True
        )

        frame_files = sorted(pathlib.Path(raw_dir).glob("frame_*.jpg"))
        frame_count = len(frame_files)
        logger.info(f"[{request_id}] {frame_count} frames extracted. Processing watermark removal...")

        # 3. Process frames
        def process_all_frames():
            for i, fp in enumerate(frame_files):
                img = Image.open(str(fp)).convert("RGB")
                cleaned = remove_notebooklm_watermark_cv2(img)
                out_path = os.path.join(clean_dir, f"frame_{i+1:04d}.png")
                cleaned.save(out_path, format="PNG")
                if (i + 1) % 50 == 0 or i == 0:
                    logger.info(f"[{request_id}] Processed {i+1}/{frame_count} frames")

        await asyncio.to_thread(process_all_frames)

        # 4. Reassemble video
        output_path = os.path.join(tmp_dir, "output.mp4")
        logger.info(f"[{request_id}] Reassembling video...")
        subprocess.run(
            ["ffmpeg", "-y",
             "-framerate", str(fps),
             "-i", os.path.join(clean_dir, "frame_%04d.png"),
             "-c:v", "libx264", "-pix_fmt", "yuv420p", "-crf", "18",
             output_path],
            check=True, capture_output=True
        )

        # 5. Upload
        key = req.output_key or f"{req.job_id}/dewatermark/video_clean.mp4"
        result_url = await upload_video_s3(output_path, key)

        elapsed = time.time() - start_time
        logger.info(f"[{request_id}] Done in {elapsed:.1f}s → {result_url}")
        return {
            "status": "success",
            "url": result_url,
            "frames_processed": frame_count,
            "duration": f"{elapsed:.2f}s",
        }

    except HTTPException:
        raise
    except Exception as e:
        logger.exception(f"[{request_id}] Error")
        raise HTTPException(500, str(e))
    finally:
        shutil.rmtree(tmp_dir, ignore_errors=True)


if __name__ == "__main__":
    import uvicorn
    # Filter out health check access logs from uvicorn
    logging.getLogger("uvicorn.access").addFilter(HealthCheckFilter())
    port = int(os.getenv("PORT", "8000"))
    uvicorn.run(app, host="0.0.0.0", port=port, log_level="info")
