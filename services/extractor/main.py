"""
keryx-extractor - Robust Video download and audio extraction service.
"""
import os
import uuid
import time
import asyncio
import logging
import tempfile
import shutil
import re
from typing import Optional, Dict, Any
from pathlib import Path
from contextlib import asynccontextmanager

import aioboto3
import botocore.client
from fastapi import FastAPI, HTTPException, Request
from pydantic import BaseModel, HttpUrl, Field

# --- Logging Setup ---
logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s [%(levelname)s] %(name)s: %(message)s",
    datefmt="%Y-%m-%d %H:%M:%S",
)
logger = logging.getLogger("keryx.extractor")
 
class HealthCheckFilter(logging.Filter):
    def filter(self, record: logging.LogRecord) -> bool:
        # Filter out noisy health check access logs
        return "/health" not in record.getMessage()

# --- Configuration ---
SERVICE_NAME = "keryx-extractor"
S3_ENDPOINT = os.getenv("S3_ENDPOINT", "https://minio-170-api.zacharie.org")
S3_ACCESS_KEY = os.getenv("AWS_ACCESS_KEY_ID")
S3_SECRET_KEY = os.getenv("AWS_SECRET_ACCESS_KEY")
S3_BUCKET = os.getenv("S3_BUCKET", "keryx")
S3_REGION = os.getenv("S3_REGION", "us-east-1")

# --- Async S3 Client Management ---
class S3Manager:
    def __init__(self):
        self.session = aioboto3.Session()

    @asynccontextmanager
    async def get_client(self):
        config = botocore.client.Config(
            signature_version=botocore.UNSIGNED,
            s3={'addressing_style': 'path'}
        )
        async with self.session.client(
            "s3",
            endpoint_url=S3_ENDPOINT,
            aws_access_key_id=S3_ACCESS_KEY,
            aws_secret_access_key=S3_SECRET_KEY,
            config=config,
            region_name=S3_REGION,
            verify=False, # Often needed for local S3/MinIO
        ) as client:
            yield client

s3_manager = S3Manager()

# --- Models ---
class ExtractRequest(BaseModel):
    url: str # HttpUrl can be too strict for some yt-dlp inputs
    job_id: str = Field(..., description="Unique ID for the job")
    audio_format: str = "wav"
    video_quality: str = "bestvideo[ext=mp4]+bestaudio[ext=m4a]/best[ext=mp4]/best"

class ExtractResponse(BaseModel):
    status: str
    video_url: str
    audio_url: str
    duration: float
    title: str
    service: str = SERVICE_NAME
    request_id: str

# --- App Lifecycle ---
@asynccontextmanager
async def lifespan(app: FastAPI):
    # Startup: Can add version checks for tools here
    logger.info(f"Starting {SERVICE_NAME}...")
    yield
    # Shutdown: Clean resources if any

app = FastAPI(title="Keryx Extractor", version="1.1.0", lifespan=lifespan)

# --- Helper Functions ---
async def run_command(cmd: list[str], request_id: str, label: str) -> str:
    logger.info(f"[{request_id}] Running {label}: {' '.join(cmd)}")
    process = await asyncio.create_subprocess_exec(
        *cmd,
        stdout=asyncio.subprocess.PIPE,
        stderr=asyncio.subprocess.PIPE
    )
    stdout, stderr = await process.communicate()
    
    if process.returncode != 0:
        error_msg = stderr.decode().strip()
        logger.error(f"[{request_id}] {label} failed with code {process.returncode}: {error_msg}")
        raise HTTPException(
            status_code=500, 
            detail=f"{label} failed: {error_msg[-500:]}"
        )
    return stdout.decode().strip()

async def upload_to_s3(local_path: str, s3_key: str, content_type: str, request_id: str) -> str:
    logger.info(f"[{request_id}] Uploading to S3: {s3_key} (Single PutObject)")
    async with s3_manager.get_client() as s3:
        with open(local_path, "rb") as f:
            await s3.put_object(
                Bucket=S3_BUCKET,
                Key=s3_key,
                Body=f,
                ContentType=content_type
            )
    return f"{S3_ENDPOINT}/{S3_BUCKET}/{s3_key}"

# --- Endpoints ---
@app.get("/health")
def health():
    return {"status": "ok", "service": SERVICE_NAME, "version": "1.1.0"}

@app.post("/extract", response_model=ExtractResponse)
async def extract(req: ExtractRequest):
    request_id = str(uuid.uuid4())[:8]
    logger.info(f"[{request_id}] NEW REQUEST | job={req.job_id} | url={req.url}")
    start_time = time.time()

    tmp_dir = tempfile.mkdtemp(prefix=f"keryx_{request_id}_")
    try:
        # -- 1. Metadata Pre-extraction ----------------------------
        logger.info(f"[{request_id}] Fetching metadata...")
        meta_cmd = [
            "yt-dlp",
            "--no-playlist",
            "--print", "%(title)s",
            "--print", "%(duration)s",
            "--no-check-certificate",
            "--js-runtimes", "nodejs",
            "--ignore-config",
            "--extractor-args", "youtube:player-client=android",
            req.url
        ]
        meta_output = await run_command(meta_cmd, request_id, "yt-dlp-metadata")
        meta_lines = meta_output.splitlines()
        title = meta_lines[0] if len(meta_lines) > 0 else "unknown"
        try:
            duration = float(meta_lines[1]) if len(meta_lines) > 1 else 0.0
        except (ValueError, IndexError):
            duration = 0.0

        # -- 2. Download Video -------------------------------------
        video_path = os.path.join(tmp_dir, "video.mp4")
        ytdlp_cmd = [
            "yt-dlp",
            "--no-playlist",
            "--ignore-config",
            "--format", f"{req.video_quality}/best",
            "--merge-output-format", "mp4",
            "--output", video_path,
            "--no-check-certificate",
            "--js-runtimes", "nodejs",
            "--extractor-args", "youtube:player-client=android",
            "--user-agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/121.0.0.0 Safari/537.36",
            req.url,
        ]
        await run_command(ytdlp_cmd, request_id, "yt-dlp-download")

        # Fallback if output filename was different
        if not os.path.exists(video_path):
            mp4_files = list(Path(tmp_dir).glob("*.mp4"))
            if not mp4_files:
                raise HTTPException(500, "Download succeeded but no MP4 file found.")
            video_path = str(mp4_files[0])

        # -- 3. Extract Audio --------------------------------------
        audio_path = os.path.join(tmp_dir, f"audio.{req.audio_format}")
        ffmpeg_cmd = [
            "ffmpeg", "-y",
            "-i", video_path,
            "-vn", # No video
            "-acodec", "pcm_s16le" if req.audio_format == "wav" else "libmp3lame",
            "-ar", "16000", # 16kHz
            "-ac", "1",     # Mono
            audio_path,
        ]
        await run_command(ffmpeg_cmd, request_id, "ffmpeg-audio")

        # -- 4. Parallel Upload to S3 ------------------------------
        video_key = f"jobs/{req.job_id}/source/video.mp4"
        audio_key = f"jobs/{req.job_id}/source/audio.{req.audio_format}"
        
        video_url, audio_url = await asyncio.gather(
            upload_to_s3(video_path, video_key, "video/mp4", request_id),
            upload_to_s3(audio_path, audio_key, f"audio/{req.audio_format}", request_id),
        )

        elapsed = time.time() - start_time
        logger.info(f"[{request_id}] PROCESSED OK: '{title}' in {elapsed:.1f}s")

        return ExtractResponse(
            status="success",
            video_url=video_url,
            audio_url=audio_url,
            duration=duration,
            title=title,
            request_id=request_id
        )

    except HTTPException:
        raise
    except Exception as e:
        logger.exception(f"[{request_id}] CRITICAL ERROR")
        raise HTTPException(status_code=500, detail=str(e))
    finally:
        shutil.rmtree(tmp_dir, ignore_errors=True)

if __name__ == "__main__":
    import uvicorn
    # Filter out health check access logs from uvicorn
    logging.getLogger("uvicorn.access").addFilter(HealthCheckFilter())
    # Use standard uvicorn worker for development
    uvicorn.run(app, host="0.0.0.0", port=8000, log_level="info")
