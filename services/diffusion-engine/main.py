import os
import io
import warnings
warnings.filterwarnings("ignore")
import uuid
import uuid as uuid_pkg
import time
import asyncio
import numpy as np
import cv2
import logging
import subprocess
import tempfile
import shutil
import pathlib
from typing import Optional
from fastapi import FastAPI, HTTPException, BackgroundTasks
from fastapi.responses import JSONResponse
from pydantic import BaseModel
import torch
from diffusers import (
    ControlNetModel,
    StableDiffusionXLControlNetImg2ImgPipeline,
    StableDiffusionXLControlNetInpaintPipeline,
    StableDiffusionXLInpaintPipeline,
    AutoPipelineForImage2Image
)
from PIL import Image
import aioboto3
import httpx
from urllib.parse import urlparse

# Configure Logging
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s [%(levelname)s] %(name)s: %(message)s',
    datefmt='%Y-%m-%d %H:%M:%S'
)
logger = logging.getLogger("keryx.diffusion")

# Optimization: Reduce CUDA fragmentation
os.environ["PYTORCH_CUDA_ALLOC_CONF"] = "expandable_segments:True,max_split_size_mb:128"

class HealthCheckFilter(logging.Filter):
    def filter(self, record: logging.LogRecord) -> bool:
        if "/health" in record.getMessage():
            record.levelno = logging.DEBUG
            record.levelname = "DEBUG"
        return True

app = FastAPI(title="Keryx Diffusion Engine")

# Configuration
S3_ENDPOINT = os.getenv("S3_ENDPOINT", "https://minio-170-api.zacharie.org")
S3_ACCESS_KEY = os.getenv("S3_ACCESS_KEY_ID") or os.getenv("AWS_ACCESS_KEY_ID")
S3_SECRET_KEY = os.getenv("S3_SECRET_ACCESS_KEY") or os.getenv("AWS_SECRET_ACCESS_KEY")
S3_BUCKET = os.getenv("S3_BUCKET", "keryx")
MODEL_ID = os.getenv("MODEL_ID", "stabilityai/sdxl-turbo")
CONTROLNET_ID = "diffusers/controlnet-canny-sdxl-1.0"
DEVICE = "cuda" if torch.cuda.is_available() else "cpu"

# Global variable for the pipeline
pipe = None
loading_error = None
is_loading = False

async def load_models():
    global pipe, loading_error, is_loading
    if is_loading or pipe is not None:
        return
    
    is_loading = True
    try:
        logger.info(f"Starting model loading on {DEVICE}...")
        torch_dtype = torch.float16 if DEVICE == "cuda" else torch.float32

        # Load ControlNet
        logger.info(f"Loading ControlNet: {CONTROLNET_ID}")
        controlnet = await asyncio.to_thread(
            ControlNetModel.from_pretrained,
            CONTROLNET_ID,
            torch_dtype=torch_dtype,
            use_safetensors=True,
            low_cpu_mem_usage=True
        )

        # Load Pipeline
        logger.info(f"Loading Pipeline: {MODEL_ID}")
        new_pipe = await asyncio.to_thread(
            StableDiffusionXLControlNetImg2ImgPipeline.from_pretrained,
            MODEL_ID,
            controlnet=controlnet,
            torch_dtype=torch_dtype,
            variant="fp16" if DEVICE == "cuda" else None,
            use_safetensors=True,
            low_cpu_mem_usage=True
        )

        if DEVICE == "cuda":
            logger.info("Enabling maximum VRAM optimizations...")
            # model_cpu_offload is much faster than sequential for 16GB VRAM
            new_pipe.enable_model_cpu_offload()
            new_pipe.enable_attention_slicing()
            new_pipe.enable_vae_slicing()
            new_pipe.enable_vae_tiling()
            torch.cuda.empty_cache()
        else:
            new_pipe.to(DEVICE)

        pipe = new_pipe
        logger.info("Models loaded and optimized successfully.")
    except Exception as e:
        logger.error(f"Failed to load models: {str(e)}", exc_info=True)
        loading_error = str(e)
    finally:
        is_loading = False

@app.on_event("startup")
async def startup_event():
    # Start loading models in the background
    asyncio.create_task(load_models())

# Brand Colors (Teamwork.com)
TW_PINK = "#FF22B1"
TW_SLATE = "#1D1C39"
TW_WHITE = "#FFFFFF"

s3_session = aioboto3.Session()

def _s3_client():
    return s3_session.client(
        "s3",
        endpoint_url=S3_ENDPOINT,
        aws_access_key_id=S3_ACCESS_KEY,
        aws_secret_access_key=S3_SECRET_KEY,
        verify=False,
    )

class StylingRequest(BaseModel):
    image_url: str
    prompt: str
    strength: float = 0.5
    guidance_scale: float = 0.0
    num_inference_steps: int = 2
    target_path: Optional[str] = None

@app.get("/health")
def health():
    if pipe is None:
        if loading_error:
            return JSONResponse(
                status_code=500,
                content={"status": "error", "message": loading_error, "service": "diffusion-engine"}
            )
        return JSONResponse(
            status_code=503,
            content={"status": "loading", "message": "Models are still loading", "service": "diffusion-engine"}
        )
    return {"status": "ok", "device": DEVICE, "model": MODEL_ID, "controlnet": CONTROLNET_ID}

async def download_image(url: str) -> Image.Image:
    if url.startswith("/") and os.path.exists(url):
        return await asyncio.to_thread(lambda: Image.open(url).convert("RGB"))

    parsed = urlparse(url)
    if "zacharie.org" in parsed.netloc or "minio" in parsed.netloc:
        parts = parsed.path.lstrip("/").split("/")
        bucket = parts[0]
        key = "/".join(parts[1:])
        async with _s3_client() as s3:
            response = await s3.get_object(Bucket=bucket, Key=key)
            data = await response["Body"].read()
        return await asyncio.to_thread(lambda: Image.open(io.BytesIO(data)).convert("RGB"))
    else:
        async with httpx.AsyncClient() as client:
            response = await client.get(url)
        return await asyncio.to_thread(lambda: Image.open(io.BytesIO(response.content)).convert("RGB"))


async def upload_image(image: Image.Image, key: str) -> str:
    def _encode():
        buf = io.BytesIO()
        image.save(buf, format="PNG")
        return buf.getvalue()

    data = await asyncio.to_thread(_encode)
    async with _s3_client() as s3:
        await s3.put_object(Bucket=S3_BUCKET, Key=key, Body=data, ContentType="image/png")
    return f"{S3_ENDPOINT}/{S3_BUCKET}/{key}"

async def download_video(url: str, dest_path: str):
    # Support local paths
    if url.startswith("/") and os.path.exists(url):
        logger.info(f"Using local video file: {url}")
        shutil.copy(url, dest_path)
        return

    parsed = urlparse(url)
    if any(h in parsed.netloc for h in ["zacharie.org", "minio", "localhost"]):
        parts = parsed.path.lstrip("/").split("/")
        bucket = parts[0]
        key = "/".join(parts[1:])
        async with _s3_client() as s3:
            await s3.download_file(bucket, key, dest_path)
    else:
        async with httpx.AsyncClient() as client:
            async with client.stream("GET", url) as response:
                with open(dest_path, "wb") as f:
                    async for chunk in response.aiter_bytes(8192):
                        f.write(chunk)

async def upload_video(path: str, key: str) -> str:
    async with _s3_client() as s3:
        await s3.upload_file(path, S3_BUCKET, key, ExtraArgs={"ContentType": "video/mp4"})
    return f"{S3_ENDPOINT}/{S3_BUCKET}/{key}"

def get_canny_image(image: Image.Image, low_threshold: int = 100, high_threshold: int = 200) -> Image.Image:
    """
    Generates a Canny edge map for ControlNet structural guidance.
    Used to maintain the layout and structure of the original slide.
    """
    # Convert PIL to numpy (OpenCV format)
    image_np = np.array(image)
    
    # Apply Canny edge detection
    edges = cv2.Canny(image_np, low_threshold, high_threshold)
    
    # ControlNet expects a 3-channel RGB image
    edges = edges[:, :, None]
    edges = np.concatenate([edges, edges, edges], axis=2)
    
    return Image.fromarray(edges)


def remove_notebooklm_watermark(image: Image.Image) -> Image.Image:
    """
    Advanced watermark removal inspired by Albonire's notebooklm-watermark-remover.
    Uses median blur difference to build a precise mask and OpenCV inpainting
    to fill the region. Surrounding textures are preserved.
    """
    # Convert PIL to BGR (OpenCV)
    img_bgr = cv2.cvtColor(np.array(image.convert("RGB")), cv2.COLOR_RGB2BGR)
    h, w = img_bgr.shape[:2]

    # 1. Define Search ROI (Bottom-Right)
    # Search in the bottom 10% and right 25% of the image
    mx, my = int(w * 0.25), int(h * 0.10)
    y0, x0 = h - my, w - mx
    roi = img_bgr[y0:h, x0:w].copy()

    # 2. Build Watermark Mask (Median Blur Difference)
    # This detects sharp features that differ from the local background
    rh, rw = roi.shape[:2]
    if rh < 5 or rw < 5:
        return image

    ksize = max(11, min(31, (min(rh, rw) // 6) | 1))
    background = cv2.medianBlur(roi, ksize)
    diff_gray = cv2.cvtColor(cv2.absdiff(roi, background), cv2.COLOR_BGR2GRAY)
    
    # Threshold to isolate the watermark glyphs
    _, binary = cv2.threshold(diff_gray, 30, 255, cv2.THRESH_BINARY)

    # Filter by Connected Components (ensure it's actually a logo, not random noise)
    num_labels, labels, stats, _ = cv2.connectedComponentsWithStats(binary, connectivity=8)
    mask = np.zeros((rh, rw), dtype=np.uint8)
    
    found_watermark = False
    for i in range(1, num_labels):
        area = stats[i, cv2.CC_STAT_AREA]
        # Skip tiny noise or massive borders
        if area < 100 or area > (rh * rw * 0.5):
            continue
        
        # Draw component pixels on mask
        mask[labels == i] = 255
        found_watermark = True

    if not found_watermark:
        logger.info("No watermark components detected in ROI, skipping reconstruction.")
        return image

    # 3. Dilate Mask to catch anti-aliasing
    kernel = cv2.getStructuringElement(cv2.MORPH_ELLIPSE, (3, 3))
    mask = cv2.dilate(mask, kernel, iterations=2)

    # 4. Inpaint the ROI
    cleaned_roi = cv2.inpaint(roi, mask, 3, cv2.INPAINT_TELEA)

    # 5. Paste back into original image
    img_bgr[y0:h, x0:w] = cleaned_roi

    # Convert back to PIL RGB
    return Image.fromarray(cv2.cvtColor(img_bgr, cv2.COLOR_BGR2RGB))


def remove_background(image: Image.Image) -> Image.Image:
    """Massive object vectorization: GaussianBlur + MORPH_CLOSE + RETR_EXTERNAL, white background."""
    import cv2
    arr = np.array(image.convert("RGB"))
    gray = cv2.cvtColor(arr, cv2.COLOR_RGB2GRAY)
    h, w = arr.shape[:2]

    blurred = cv2.GaussianBlur(gray, (5, 5), 0)
    _, thresh = cv2.threshold(blurred, 245, 255, cv2.THRESH_BINARY_INV)

    # Weld nearby elements into solid blocks
    kernel = np.ones((15, 15), np.uint8)
    morphed = cv2.morphologyEx(thresh, cv2.MORPH_CLOSE, kernel)

    contours, _ = cv2.findContours(morphed, cv2.RETR_EXTERNAL, cv2.CHAIN_APPROX_SIMPLE)
    min_area = h * w * 0.10
    mask = np.zeros((h, w), dtype=np.uint8)
    for cnt in contours:
        if cv2.contourArea(cnt) > min_area:
            cv2.drawContours(mask, [cnt], -1, 255, thickness=-1)

    # Smooth mask edges
    mask = cv2.GaussianBlur(mask, (7, 7), 0)
    _, mask = cv2.threshold(mask, 127, 255, cv2.THRESH_BINARY)

    coords = cv2.findNonZero(mask)
    if coords is None:
        return image
    x, y, cw, ch = cv2.boundingRect(coords)
    roi = arr[y:y+ch, x:x+cw]
    mask_roi = mask[y:y+ch, x:x+cw]
    result = np.full((ch, cw, 3), 255, dtype=np.uint8)
    result[mask_roi == 255] = roi[mask_roi == 255]
    return Image.fromarray(result)


# Keep old name as alias for video pipeline compatibility
def inpaint_bottom_right(_pipe, image: Image.Image) -> Image.Image:
    return remove_notebooklm_watermark(image)

class InpaintRequest(BaseModel):
    image_url: str
    mask_url: str
    prompt: str
    strength: float = 0.9
    controlnet_conditioning_scale: float = 0.5
    num_inference_steps: int = 30
    target_path: Optional[str] = None

class CleanRequest(BaseModel):
    image_url: str
    job_id: str
    target_path: Optional[str] = None

class VideoCleanRequest(BaseModel):
    video_url: str
    target_path: Optional[str] = None
    fps_override: Optional[float] = None

@app.post("/style")
async def style_image(request: StylingRequest):
    request_id = str(uuid_pkg.uuid4())[:8]
    logger.info(f"[{request_id}] Received styling request for: {request.image_url}")
    start_time = time.time()
    try:
        # 1. Download and Prepare
        init_image = await download_image(request.image_url)
        logger.info(f"[{request_id}] Downloaded image. Original size: {init_image.size}")

        init_image = init_image.resize((512, 512))

        # Extract Canny edges for structural preservation
        control_image = get_canny_image(init_image)
        logger.info(f"[{request_id}] Generated Canny control map for structural preservation.")

        # 2. Refine Prompt with Teamwork Colors
        brand_prompt = f"{request.prompt}. Teamwork brand aesthetic: vibrant pink ({TW_PINK}), deep slate ({TW_SLATE}), and clean white ({TW_WHITE}) highlights. Professional SaaS presentation style, high quality, glassmorphism."
        logger.info(f"[{request_id}] Using prompt: {brand_prompt}")

        # 3. Run Inference
        logger.info(f"[{request_id}] Starting SDXL Turbo inference (Steps: {request.num_inference_steps}, Strength: {request.strength})...")
        if DEVICE == "cuda":
            torch.cuda.empty_cache()
            
        with torch.inference_mode():
            images = pipe(
                brand_prompt,
                image=init_image,
                control_image=control_image,
                strength=request.strength,
                guidance_scale=request.guidance_scale,
                num_inference_steps=request.num_inference_steps,
                controlnet_conditioning_scale=0.5 # Balance between prompt and structure
            ).images

        stylized_image = images[0]

        # 4. Upload result
        if not request.target_path:
            filename = f"styled_{uuid_pkg.uuid4()}.jpg"
            target_key = f"{request.job_id}/diffusion-engine/styled/{filename}"
        else:
            target_key = request.target_path

        logger.info(f"[{request_id}] Uploading result to S3: {target_key}")
        result_url = await upload_image(stylized_image, target_key)

        duration = time.time() - start_time
        logger.info(f"[{request_id}] Request finished in {duration:.2f}s. Result: {result_url}")

        return {
            "status": "success",
            "url": result_url,
            "prompt": brand_prompt
        }

    except Exception as e:
        logger.error(f"[{request_id}] Error during styling: {str(e)}", exc_info=True)
        raise HTTPException(status_code=500, detail=str(e))

@app.post("/clean_watermark")
async def clean_watermark(request: CleanRequest):
    request_id = str(uuid_pkg.uuid4())[:8]
    logger.info(f"[{request_id}] Received watermark cleaning request for: {request.image_url}")
    start_time = time.time()
    try:
        init_image = await download_image(request.image_url)
        logger.info(f"[{request_id}] Image size: {init_image.size}")

        cleaned_image = remove_notebooklm_watermark(init_image)

        if request.target_path and request.target_path.startswith("/"):
            os.makedirs(os.path.dirname(request.target_path), exist_ok=True)
            cleaned_image.save(request.target_path, format="PNG")
            result_url = f"file://{request.target_path}"
            target_key = request.target_path
        else:
            target_key = request.target_path or f"diffusion-engine/cleaned/{uuid_pkg.uuid4()}.png"
            result_url = await upload_image(cleaned_image, target_key)

        duration = time.time() - start_time
        logger.info(f"[{request_id}] Done in {duration:.2f}s. Result: {result_url}")
        return {"status": "success", "url": result_url, "target": target_key}

    except Exception as e:
        logger.error(f"[{request_id}] Error: {str(e)}", exc_info=True)
        raise HTTPException(status_code=500, detail=str(e))

@app.post("/clean_video_watermark")
async def clean_video_watermark(request: VideoCleanRequest):
    request_id = str(uuid_pkg.uuid4())[:8]
    logger.info(f"[{request_id}] Received video watermark cleaning request for: {request.video_url}")
    start_time = time.time()

    tmp_dir = tempfile.mkdtemp(prefix=f"keryx_video_{request_id}_")
    try:
        # 1. Download Video
        video_path = f"{tmp_dir}/input.mp4"
        logger.info(f"[{request_id}] Downloading video to {video_path}...")
        await download_video(request.video_url, video_path)

        if not os.path.exists(video_path) or os.path.getsize(video_path) == 0:
            raise Exception(f"Video download failed or file empty: {video_path}")

        # 2. Extract ALL Frames (Phase 1)
        # Using OpenCV with FFMPEG backend, falling back to FFmpeg CLI if needed for robustness
        raw_frames_dir = f"{tmp_dir}/raw_frames"
        cleaned_dir = f"{tmp_dir}/cleaned"
        os.makedirs(raw_frames_dir, exist_ok=True)
        os.makedirs(cleaned_dir, exist_ok=True)

        frame_count = 0
        success = False
        try:
            logger.info(f"[{request_id}] Attempting extraction via OpenCV (FFMPEG backend)...")
            cap = cv2.VideoCapture(video_path, cv2.CAP_FFMPEG)
            if not cap.isOpened():
                raise Exception("OpenCV VideoCapture could not open the file.")

            fps = request.fps_override or cap.get(cv2.CAP_PROP_FPS) or 24
            total_frames = int(cap.get(cv2.CAP_PROP_FRAME_COUNT))
            logger.info(f"[{request_id}] Video properties: FPS={fps}, Total Frames={total_frames}")

            while cap.isOpened():
                ret, frame = cap.read()
                if not ret:
                    break
                frame_count += 1
                frame_filename = f"{raw_frames_dir}/frame_{frame_count:04d}.jpg"
                cv2.imwrite(frame_filename, frame)
                if frame_count % 10 == 0:
                    logger.info(f"[{request_id}] OpenCV: Extracted {frame_count}/{total_frames if total_frames > 0 else '?'} frames...")

            cap.release()
            if frame_count > 0:
                success = True
                logger.info(f"[{request_id}] OpenCV extraction successful. Total: {frame_count} frames.")

        except Exception as e:
            logger.warning(f"[{request_id}] OpenCV extraction issue: {str(e)}. Falling back to FFmpeg CLI.")
            frame_count = 0

        if not success:
            logger.info(f"[{request_id}] Running FFmpeg CLI for robust frame extraction...")
            try:
                # Use sub-process to call ffmpeg directly (very reliable)
                subprocess.run([
                    "ffmpeg", "-y", "-i", video_path,
                    "-q:v", "2",
                    os.path.join(raw_frames_dir, "frame_%04d.jpg")
                ], check=True, capture_output=True)

                extracted_files = list(pathlib.Path(raw_frames_dir).glob("*.jpg"))
                frame_count = len(extracted_files)
                fps = 24 # Baseline fallback
                logger.info(f"[{request_id}] FFmpeg CLI extraction successful. Total: {frame_count} frames.")
                if frame_count > 0:
                    success = True
            except Exception as fe:
                logger.error(f"[{request_id}] Critical Failure: All extraction methods failed. {str(fe)}")

        if frame_count == 0:
            raise Exception("Fatal: Failed to recover any frames from video source.")

        # 3. STAGE 2: Upload ALL Raw Frames to S3 concurrently
        logger.info(f"[{request_id}] Uploading {frame_count} raw frames to S3 (diffusion-engine/raw_frames/{request_id}/)...")
        async def _upload_frame(i: int):
            filename = f"frame_{i:04d}.jpg"
            s3_key = f"raw_frames/{request_id}/{filename}"
            async with _s3_client() as s3:
                await s3.upload_file(f"{raw_frames_dir}/{filename}", S3_BUCKET, s3_key)

        await asyncio.gather(*[_upload_frame(i) for i in range(1, frame_count + 1)])
        logger.info(f"[{request_id}] All raw frames successfully pushed to S3.")

        # 4. Process each frame (CV2 background fill, no SDXL needed)
        logger.info(f"[{request_id}] Starting watermark removal (CV2 background fill)...")

        # 5. Process each frame (Downloading back from S3 as requested)
        for i in range(1, frame_count + 1):
            filename = f"frame_{i:04d}.jpg"
            s3_key = f"raw_frames/{request_id}/{filename}"
            local_raw_path = f"{tmp_dir}/s3_downloaded_{filename}"

            # Pull from S3
            async with _s3_client() as s3:
                await s3.download_file(S3_BUCKET, s3_key, local_raw_path)

            if i % 10 == 0 or i == 1:
                logger.info(f"[{request_id}] Processing frame {i}/{frame_count} (Downloaded from S3)...")

            # Load frame and prepare
            init_image = Image.open(local_raw_path).convert("RGB")

            final_frame = inpaint_bottom_right(None, init_image)

            # Save cleaned frame (PNG pour éviter la compression JPEG)
            frame_output_filename = f"{cleaned_dir}/frame_{i:04d}.png"
            final_frame.save(frame_output_filename, format="PNG")

            # Cleanup downloaded raw frame to save disk
            if os.path.exists(local_raw_path):
                os.remove(local_raw_path)

        # 6. Assemble Video using FFmpeg
        output_video = f"{tmp_dir}/output.mp4"
        logger.info(f"[{request_id}] Re-assembling video with FFmpeg...")
        assemble_cmd = [
            "ffmpeg", "-y", "-framerate", str(fps),
            "-i", f"{cleaned_dir}/frame_%04d.png",
            "-c:v", "libx264", "-pix_fmt", "yuv420p",
            "-crf", "18",
            output_video
        ]
        subprocess.run(assemble_cmd, check=True, capture_output=True)

        # 7. Upload Final Result
        if not request.target_path:
            final_filename = f"video_cleaned_{uuid_pkg.uuid4()}.mp4"
            target_key = f"diffusion-engine/cleaned_videos/{final_filename}"
        else:
            target_key = request.target_path

        logger.info(f"[{request_id}] Uploading final video to S3: {target_key}")
        result_url = await upload_video(output_video, target_key)

        duration = time.time() - start_time
        logger.info(f"[{request_id}] Video cleaning finished in {duration:.2f}s. Result: {result_url}")

        return {
            "status": "success",
            "url": result_url,
            "frames_processed": frame_count,
            "duration": f"{duration:.2f}s",
            "raw_frames_s3": f"diffusion-engine/raw_frames/{request_id}/"
        }

    except Exception as e:
        logger.error(f"[{request_id}] Error during video cleaning: {str(e)}", exc_info=True)
        raise HTTPException(status_code=500, detail=str(e))
    finally:
        # Cleanup
        shutil.rmtree(tmp_dir, ignore_errors=True)

class RemoveBgRequest(BaseModel):
    image_url: str
    target_path: Optional[str] = None

@app.post("/remove_background")
async def remove_background_endpoint(request: RemoveBgRequest):
    request_id = str(uuid_pkg.uuid4())[:8]
    logger.info(f"[{request_id}] Remove background request for: {request.image_url}")
    start_time = time.time()
    try:
        init_image = await download_image(request.image_url)
        result_image = await asyncio.to_thread(remove_background, init_image)

        target_key = request.target_path or f"diffusion-engine/nobg/{uuid_pkg.uuid4()}.png"
        if target_key.startswith("/"):
            os.makedirs(os.path.dirname(target_key), exist_ok=True)
            result_image.save(target_key, format="PNG")
            result_url = f"file://{target_key}"
        else:
            result_url = await upload_image(result_image, target_key)

        duration = time.time() - start_time
        logger.info(f"[{request_id}] Done in {duration:.2f}s. Result: {result_url}")
        return {"status": "success", "url": result_url}
    except Exception as e:
        logger.error(f"[{request_id}] Error: {str(e)}", exc_info=True)
        raise HTTPException(status_code=500, detail=str(e))


async def inpaint_image(request: InpaintRequest):
    request_id = str(uuid_pkg.uuid4())[:8]
    logger.info(f"[{request_id}] Received inpaint request for: {request.image_url}")
    start_time = time.time()
    try:
        # 1. Download and Prepare
        init_image = (await download_image(request.image_url)).resize((1024, 1024))
        mask_image = (await download_image(request.mask_url)).resize((1024, 1024))

        # Extract Canny edges for structural preservation
        control_image = get_canny_image(init_image)
        logger.info(f"[{request_id}] Generated Canny control map.")

        # 2. Setup Inpaint Pipeline (convert from Img2Img to Inpaint without reloading weights if possible)
        # For simplicity and to avoid VRAM issues, we can use the same pipe if we're careful
        # But StableDiffusionXLControlNetInpaintPipeline is a separate class.
        # We can create it using the same components to save memory.
        inpaint_pipe = StableDiffusionXLControlNetInpaintPipeline(
            vae=pipe.vae,
            text_encoder=pipe.text_encoder,
            text_encoder_2=pipe.text_encoder_2,
            tokenizer=pipe.tokenizer,
            tokenizer_2=pipe.tokenizer_2,
            unet=pipe.unet,
            controlnet=pipe.controlnet,
            scheduler=pipe.scheduler,
            force_zeros_for_empty_prompt=pipe.config.force_zeros_for_empty_prompt,
            add_watermarker=getattr(pipe, "add_watermarker", None)
        )
        if DEVICE != "cuda":
            inpaint_pipe.to(DEVICE)
        else:
            inpaint_pipe.enable_model_cpu_offload()

        # 3. Run Inference
        logger.info(f"[{request_id}] Starting SDXL Inpaint inference...")
        with torch.inference_mode():
            images = inpaint_pipe(
                prompt=request.prompt,
                negative_prompt="low quality, blurry, distorted text, ugly, messy",
                image=init_image,
                mask_image=mask_image,
                control_image=control_image,
                strength=request.strength,
                num_inference_steps=request.num_inference_steps,
                controlnet_conditioning_scale=request.controlnet_conditioning_scale
            ).images

        inpainted_image = images[0]

        # 4. Upload result
        if not request.target_path:
            filename = f"inpainted_{uuid_pkg.uuid4()}.jpg"
            target_key = f"diffusion-engine/inpainted/{filename}"
        else:
            target_key = request.target_path

        logger.info(f"[{request_id}] Uploading result to S3: {target_key}")
        result_url = await upload_image(inpainted_image, target_key)

        duration = time.time() - start_time
        logger.info(f"[{request_id}] Request finished in {duration:.2f}s. Result: {result_url}")

        return {
            "status": "success",
            "url": result_url,
            "prompt": request.prompt
        }

    except Exception as e:
        logger.error(f"[{request_id}] Error during inpainting: {str(e)}", exc_info=True)
        raise HTTPException(status_code=500, detail=str(e))

if __name__ == "__main__":
    import uvicorn
    # Filter out health check access logs from uvicorn
    logging.getLogger("uvicorn.access").addFilter(HealthCheckFilter())
    uvicorn.run(app, host="0.0.0.0", port=8000, log_level="info")
