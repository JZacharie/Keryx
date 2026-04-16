"""
keryx-video-composer — Assemblage vidéo à partir de slides + audio.

POST /compose        : slides images + audio → vidéo finale MP4
POST /concat_audio   : segments WAV → audio fusionné
POST /detect_slides  : vidéo → keyframes + timestamps (scene detection ffmpeg)
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
from typing import Optional, List

import aioboto3
import httpx
from fastapi import FastAPI, HTTPException
from pydantic import BaseModel
from urllib.parse import urlparse

logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s [%(levelname)s] %(name)s: %(message)s",
    datefmt="%Y-%m-%d %H:%M:%S",
)
logger = logging.getLogger("keryx.video_composer")

class HealthCheckFilter(logging.Filter):
    def filter(self, record: logging.LogRecord) -> bool:
        if "/health" in record.getMessage():
            record.levelno = logging.DEBUG
            record.levelname = "DEBUG"
        return True

app = FastAPI(title="Keryx Video Composer", version="1.0.0")

SERVICE_NAME = "keryx-video-composer"
S3_ENDPOINT = os.getenv("S3_ENDPOINT", "https://minio-170-api.zacharie.org")
S3_ACCESS_KEY = os.getenv("S3_ACCESS_KEY_ID") or os.getenv("AWS_ACCESS_KEY_ID")
S3_SECRET_KEY = os.getenv("S3_SECRET_ACCESS_KEY") or os.getenv("AWS_SECRET_ACCESS_KEY")
S3_BUCKET = os.getenv("S3_BUCKET", "keryx")

s3_session = aioboto3.Session()


def _s3_client():
    return s3_session.client(
        "s3",
        endpoint_url=S3_ENDPOINT,
        aws_access_key_id=S3_ACCESS_KEY,
        aws_secret_access_key=S3_SECRET_KEY,
        verify=False,
    )


# ── S3 helpers ────────────────────────────────────────────────────────────────

async def download_file(url: str, dest: str):
    """Télécharge depuis S3 ou HTTP vers un fichier local."""
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
        async with httpx.AsyncClient(verify=False, timeout=120) as client:
            async with client.stream("GET", url) as resp:
                with open(dest, "wb") as f:
                    async for chunk in resp.aiter_bytes(8192):
                        f.write(chunk)


async def upload_file(local_path: str, key: str, content_type: str = "video/mp4") -> str:
    async with _s3_client() as s3:
        await s3.upload_file(
            local_path, S3_BUCKET, key,
            ExtraArgs={"ContentType": content_type}
        )
    return f"{S3_ENDPOINT}/{S3_BUCKET}/{key}"


# ── ffmpeg helpers ────────────────────────────────────────────────────────────

def run_ffmpeg(*args, check=True):
    """Lance ffmpeg avec stderr capturé, raise HTTPException si erreur."""
    cmd = ["ffmpeg", "-y"] + list(args)
    result = subprocess.run(cmd, capture_output=True)
    if check and result.returncode != 0:
        raise HTTPException(500, f"ffmpeg error: {result.stderr.decode()[-1000:]}")
    return result


# ── API Models ────────────────────────────────────────────────────────────────

class SlideInput(BaseModel):
    image_url: str
    duration: float  # durée en secondes pour cette slide


class ComposeRequest(BaseModel):
    job_id: str
    slides: List[SlideInput]
    audio_url: Optional[str] = None    # Audio overlay global (optionnel)
    intro_url: Optional[str] = None    # Clip intro à préfixer (optionnel)
    output_key: Optional[str] = None
    fps: int = 24


class ConcatAudioRequest(BaseModel):
    job_id: str
    segments: List[str]  # URLs S3 des segments WAV
    output_key: Optional[str] = None


class DetectSlidesRequest(BaseModel):
    job_id: str
    video_url: str
    scene_threshold: float = 0.3  # Seuil de détection de changement de scène
    output_prefix: Optional[str] = None  # Prefix S3 pour les frames extraites


class SlideFrame(BaseModel):
    index: int
    timestamp: float
    image_url: str


class DetectSlidesResponse(BaseModel):
    status: str
    slides: List[SlideFrame]
    service: str = SERVICE_NAME


# ── Endpoints ─────────────────────────────────────────────────────────────────

@app.get("/health")
def health():
    return {"status": "ok", "service": SERVICE_NAME, "version": "1.0.0"}


@app.post("/compose")
async def compose(req: ComposeRequest):
    request_id = str(uuid.uuid4())[:8]
    logger.info(f"[{request_id}] Compose job={req.job_id} slides={len(req.slides)} audio={bool(req.audio_url)}")
    start_time = time.time()

    tmp_dir = tempfile.mkdtemp(prefix=f"keryx_vc_{request_id}_")
    try:
        frames_dir = os.path.join(tmp_dir, "frames")
        os.makedirs(frames_dir, exist_ok=True)

        # 1. Télécharger toutes les images en parallèle
        logger.info(f"[{request_id}] Downloading {len(req.slides)} slide images...")
        local_frames = []
        async def dl_frame(i, slide):
            ext = os.path.splitext(slide.image_url)[-1] or ".jpg"
            dest = os.path.join(frames_dir, f"frame_{i:04d}{ext}")
            await download_file(slide.image_url, dest)
            return dest

        local_frames = await asyncio.gather(*[dl_frame(i, s) for i, s in enumerate(req.slides)])

        # 2. Créer un fichier concat list pour ffmpeg
        # Chaque image est convertie en segment vidéo avec sa durée
        segments_dir = os.path.join(tmp_dir, "segments")
        os.makedirs(segments_dir, exist_ok=True)

        concat_list_path = os.path.join(tmp_dir, "concat.txt")
        segment_paths = []

        logger.info(f"[{request_id}] Building video segments...")
        for i, (frame_path, slide) in enumerate(zip(local_frames, req.slides)):
            seg_path = os.path.join(segments_dir, f"seg_{i:04d}.mp4")
            run_ffmpeg(
                "-loop", "1",
                "-i", frame_path,
                "-t", str(slide.duration),
                "-vf", f"scale=1920:1080:force_original_aspect_ratio=decrease,pad=1920:1080:(ow-iw)/2:(oh-ih)/2",
                "-c:v", "libx264", "-pix_fmt", "yuv420p",
                "-r", str(req.fps),
                seg_path
            )
            segment_paths.append(seg_path)

        # 3. Concaténer tous les segments
        with open(concat_list_path, "w") as f:
            for sp in segment_paths:
                f.write(f"file '{sp}'\n")

        silent_video = os.path.join(tmp_dir, "silent.mp4")
        run_ffmpeg(
            "-f", "concat", "-safe", "0",
            "-i", concat_list_path,
            "-c", "copy",
            silent_video
        )

        # 4. Audio overlay si fourni
        final_video = os.path.join(tmp_dir, "final.mp4")
        if req.audio_url:
            audio_path = os.path.join(tmp_dir, "audio.wav")
            logger.info(f"[{request_id}] Downloading audio track...")
            await download_file(req.audio_url, audio_path)

            run_ffmpeg(
                "-i", silent_video,
                "-i", audio_path,
                "-c:v", "copy",
                "-c:a", "aac",
                "-shortest",
                final_video
            )
        else:
            shutil.copy(silent_video, final_video)

        # 5. Préfixer intro si fournie
        output_video = final_video
        if req.intro_url:
            intro_path = os.path.join(tmp_dir, "intro.mp4")
            logger.info(f"[{request_id}] Downloading intro clip...")
            await download_file(req.intro_url, intro_path)

            intro_concat = os.path.join(tmp_dir, "intro_concat.txt")
            with_intro = os.path.join(tmp_dir, "with_intro.mp4")
            with open(intro_concat, "w") as f:
                f.write(f"file '{intro_path}'\n")
                f.write(f"file '{final_video}'\n")

            run_ffmpeg(
                "-f", "concat", "-safe", "0",
                "-i", intro_concat,
                "-c", "copy",
                with_intro
            )
            output_video = with_intro

        # 6. Upload
        key = req.output_key or f"jobs/{req.job_id}/exports/composed_{uuid.uuid4()}.mp4"
        result_url = await upload_file(output_video, key)

        elapsed = time.time() - start_time
        logger.info(f"[{request_id}] Done in {elapsed:.1f}s → {result_url}")
        return {"status": "success", "url": result_url, "duration": f"{elapsed:.2f}s"}

    except HTTPException:
        raise
    except Exception as e:
        logger.exception(f"[{request_id}] Error")
        raise HTTPException(500, str(e))
    finally:
        shutil.rmtree(tmp_dir, ignore_errors=True)


@app.post("/concat_audio")
async def concat_audio(req: ConcatAudioRequest):
    request_id = str(uuid.uuid4())[:8]
    logger.info(f"[{request_id}] concat_audio job={req.job_id} segments={len(req.segments)}")
    start_time = time.time()

    tmp_dir = tempfile.mkdtemp(prefix=f"keryx_ca_{request_id}_")
    try:
        seg_dir = os.path.join(tmp_dir, "segs")
        os.makedirs(seg_dir, exist_ok=True)

        # Download all segments in parallel
        async def dl_seg(i, url):
            dest = os.path.join(seg_dir, f"seg_{i:04d}.wav")
            await download_file(url, dest)
            return dest

        local_segs = await asyncio.gather(*[dl_seg(i, url) for i, url in enumerate(req.segments)])

        # Concat with ffmpeg
        concat_list = os.path.join(tmp_dir, "concat.txt")
        with open(concat_list, "w") as f:
            for sp in local_segs:
                f.write(f"file '{sp}'\n")

        merged_wav = os.path.join(tmp_dir, "merged.wav")
        run_ffmpeg(
            "-f", "concat", "-safe", "0",
            "-i", concat_list,
            "-c", "copy",
            merged_wav
        )

        key = req.output_key or f"jobs/{req.job_id}/audio/merged_{uuid.uuid4()}.wav"
        result_url = await upload_file(merged_wav, key, content_type="audio/wav")

        elapsed = time.time() - start_time
        logger.info(f"[{request_id}] Merged {len(req.segments)} segments in {elapsed:.1f}s → {result_url}")
        return {"status": "success", "url": result_url, "duration": f"{elapsed:.2f}s"}

    except HTTPException:
        raise
    except Exception as e:
        logger.exception(f"[{request_id}] Error")
        raise HTTPException(500, str(e))
    finally:
        shutil.rmtree(tmp_dir, ignore_errors=True)


@app.post("/detect_slides", response_model=DetectSlidesResponse)
async def detect_slides(req: DetectSlidesRequest):
    """Détecte les changements de scène dans une vidéo et extrait les keyframes."""
    request_id = str(uuid.uuid4())[:8]
    logger.info(f"[{request_id}] detect_slides job={req.job_id} threshold={req.scene_threshold}")
    start_time = time.time()

    tmp_dir = tempfile.mkdtemp(prefix=f"keryx_ds_{request_id}_")
    try:
        # Download video
        video_path = os.path.join(tmp_dir, "video.mp4")
        await download_file(req.video_url, video_path)

        # ffmpeg scene detection — extrait uniquement les keyframes
        frames_dir = os.path.join(tmp_dir, "frames")
        os.makedirs(frames_dir, exist_ok=True)

        logger.info(f"[{request_id}] Running scene detection...")
        # -vf "select=..." sélectionne uniquement les frames de changement de scène
        vf_filter = f"select='gt(scene,{req.scene_threshold})',showinfo"
        proc = subprocess.run(
            ["ffmpeg", "-y", "-i", video_path,
             "-vf", vf_filter,
             "-vsync", "vfr", "-q:v", "2",
             os.path.join(frames_dir, "frame_%04d.jpg")],
            capture_output=True, text=True
        )

        # Parse timestamps depuis stderr (ffmpeg showinfo)
        timestamps = []
        for line in proc.stderr.splitlines():
            if "showinfo" in line and "pts_time:" in line:
                try:
                    idx = line.find("pts_time:")
                    rest = line[idx + 9:]
                    ts = float(rest.split()[0])
                    timestamps.append(ts)
                except (ValueError, IndexError):
                    pass

        import glob
        frame_files = sorted(glob.glob(os.path.join(frames_dir, "frame_*.jpg")))
        logger.info(f"[{request_id}] {len(frame_files)} keyframes detected")

        # Upload frames to S3 and build response
        prefix = req.output_prefix or f"jobs/{req.job_id}/slides"

        async def upload_frame(i, fp, ts):
            key = f"{prefix}/frame_{i:04d}.jpg"
            url = await upload_file(fp, key, content_type="image/jpeg")
            return SlideFrame(index=i, timestamp=ts, image_url=url)

        pairs = list(zip(frame_files, timestamps[:len(frame_files)]))
        slides = await asyncio.gather(*[upload_frame(i, fp, ts) for i, (fp, ts) in enumerate(pairs)])

        elapsed = time.time() - start_time
        logger.info(f"[{request_id}] Done in {elapsed:.1f}s")
        return DetectSlidesResponse(status="success", slides=list(slides))

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
    uvicorn.run(app, host="0.0.0.0", port=8000, log_level="info")
