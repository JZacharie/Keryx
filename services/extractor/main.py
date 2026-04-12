"""
keryx-extractor - Video download and audio extraction service.
POST /extract : yt-dlp -> S3 (video + audio)
"""
import os
import uuid
import time
import asyncio
import logging
import tempfile
import subprocess
import shutil
from typing import Optional
from pathlib import Path

import aioboto3
from fastapi import FastAPI, HTTPException, BackgroundTasks
from pydantic import BaseModel

logging.basicConfig(
    level=logging.DEBUG,
    format="%(asctime)s [%(levelname)s] %(name)s: %(message)s",
    datefmt="%Y-%m-%d %H:%M:%S",
)
logger = logging.getLogger("keryx.extractor")

app = FastAPI(title="Keryx Extractor", version="1.0.0")

SERVICE_NAME = "keryx-extractor"
S3_ENDPOINT = os.getenv("S3_ENDPOINT", "https://minio-170-api.zacharie.org")
S3_ACCESS_KEY = os.getenv("AWS_ACCESS_KEY_ID")
S3_SECRET_KEY = os.getenv("AWS_SECRET_ACCESS_KEY")
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


async def upload_file(local_path: str, s3_key: str, content_type: str) -> str:
    async with _s3_client() as s3:
        await s3.upload_file(
            local_path,
            S3_BUCKET,
            s3_key,
            ExtraArgs={"ContentType": content_type},
        )
    return f"{S3_ENDPOINT}/{S3_BUCKET}/{s3_key}"


class ExtractRequest(BaseModel):
    url: str
    job_id: str
    # Optional: force output format
    audio_format: str = "wav"
    # Optional: limit video quality to save time
    video_quality: str = "bestvideo[ext=mp4]+bestaudio[ext=m4a]/best[ext=mp4]/best"


class ExtractResponse(BaseModel):
    status: str
    video_url: str
    audio_url: str
    duration: float
    title: str
    service: str = SERVICE_NAME


@app.get("/health")
def health():
    return {"status": "ok", "service": SERVICE_NAME, "version": "1.0.0"}


@app.post("/extract", response_model=ExtractResponse)
async def extract(req: ExtractRequest):
    request_id = str(uuid.uuid4())[:8]
    logger.info(f"[{request_id}] Extract request for job={req.job_id} url={req.url}")
    start_time = time.time()

    tmp_dir = tempfile.mkdtemp(prefix=f"keryx_extract_{request_id}_")
    try:
        # -- 1. Download with yt-dlp ----------------------------------
        video_path = os.path.join(tmp_dir, "video.mp4")
        audio_wav = os.path.join(tmp_dir, "audio.wav")

        logger.info(f"[{request_id}] Downloading video with yt-dlp...")
        ytdlp_cmd = [
            "yt-dlp",
            "--no-playlist",
            "--format", req.video_quality,
            "--merge-output-format", "mp4",
            "--output", video_path,
            "--print-to-file", "%(title)s", os.path.join(tmp_dir, "title.txt"),
            "--print-to-file", "%(duration)s", os.path.join(tmp_dir, "duration.txt"),
            req.url,
        ]
        result = await asyncio.to_thread(
            subprocess.run, ytdlp_cmd, capture_output=True, text=True
        )
        if result.returncode != 0:
            raise HTTPException(500, f"yt-dlp failed: {result.stderr[-500:]}")

        if not os.path.exists(video_path):
            # Try to find file with alternative name
            mp4_files = list(Path(tmp_dir).glob("*.mp4"))
            if not mp4_files:
                raise HTTPException(500, "yt-dlp did not produce any mp4 file")
            video_path = str(mp4_files[0])

        # Metadata reading
        title = "unknown"
        duration = 0.0
        title_file = os.path.join(tmp_dir, "title.txt")
        duration_file = os.path.join(tmp_dir, "duration.txt")
        if os.path.exists(title_file):
            title = open(title_file).read().strip()
        if os.path.exists(duration_file):
            try:
                duration = float(open(duration_file).read().strip())
            except ValueError:
                pass

        logger.info(f"[{request_id}] Downloaded: '{title}' ({duration}s)")

        # -- 2. Audio extraction with ffmpeg --------------------------------
        logger.info(f"[{request_id}] Extracting audio to WAV...")
        ffmpeg_cmd = [
            "ffmpeg", "-y",
            "-i", video_path,
            "-vn",
            "-acodec", "pcm_s16le",
            "-ar", "16000",
            "-ac", "1",
            audio_wav,
        ]
        result = await asyncio.to_thread(
            subprocess.run, ffmpeg_cmd, capture_output=True
        )
        if result.returncode != 0:
            raise HTTPException(500, f"ffmpeg audio extraction failed: {result.stderr.decode()[-500:]}")

        # -- 3. Upload to S3 ----------------------------------------------
        logger.info(f"[{request_id}] Uploading video and audio to S3...")
        video_key = f"jobs/{req.job_id}/source/video.mp4"
        audio_key = f"jobs/{req.job_id}/source/audio.wav"

        video_url, audio_url = await asyncio.gather(
            upload_file(video_path, video_key, "video/mp4"),
            upload_file(audio_wav, audio_key, "audio/wav"),
        )

        elapsed = time.time() - start_time
        logger.info(f"[{request_id}] Done in {elapsed:.1f}s. video={video_url} audio={audio_url}")

        return ExtractResponse(
            status="success",
            video_url=video_url,
            audio_url=audio_url,
            duration=duration,
            title=title,
        )

    except HTTPException:
        raise
    except Exception as e:
        logger.exception(f"[{request_id}] Unexpected error")
        raise HTTPException(500, str(e))
    finally:
        shutil.rmtree(tmp_dir, ignore_errors=True)


if __name__ == "__main__":
    import uvicorn
    uvicorn.run(app, host="0.0.0.0", port=8010)
