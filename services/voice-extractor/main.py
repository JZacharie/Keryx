"""
keryx-voice-extractor — Transcription STT (Whisper) + Traduction (Ollama ou deep-translator).

POST /transcribe  : Audio S3 → segments JSON timestampés
POST /translate   : Segments → segments traduits
"""
import os
import io
import uuid
import time
import asyncio
import logging
import tempfile
import shutil
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
logger = logging.getLogger("keryx.voice_extractor")

class HealthCheckFilter(logging.Filter):
    def filter(self, record: logging.LogRecord) -> bool:
        # Suppress /health from logs if they contain it
        return "/health" not in record.getMessage()

from contextlib import asynccontextmanager

# ── Whisper — chargement ───────────────────────────────────────────────────
_whisper_model = None
_whisper_lock = asyncio.Lock()


async def get_whisper():
    global _whisper_model
    if _whisper_model is None:
        async with _whisper_lock:
            if _whisper_model is None:
                import torch
                import whisper
                device = "cuda" if torch.cuda.is_available() else "cpu"
                logger.info(f"Loading Whisper model '{WHISPER_MODEL}' on {device}...")
                _whisper_model = await asyncio.to_thread(
                    whisper.load_model, WHISPER_MODEL, device=device, download_root=WHISPER_CACHE_DIR
                )
                logger.info("Whisper model loaded.")
    return _whisper_model


@asynccontextmanager
async def lifespan(app: FastAPI):
    # Load model at startup
    try:
        await get_whisper()
    except Exception as e:
        logger.error(f"Failed to load Whisper model at startup: {e}")
    yield


app = FastAPI(title="Keryx Voice Extractor", version="1.0.0", lifespan=lifespan)

SERVICE_NAME = "keryx-voice-extractor"
S3_ENDPOINT = os.getenv("S3_ENDPOINT", "https://minio-170-api.zacharie.org")
S3_ACCESS_KEY = os.getenv("S3_ACCESS_KEY_ID") or os.getenv("AWS_ACCESS_KEY_ID")
S3_SECRET_KEY = os.getenv("S3_SECRET_ACCESS_KEY") or os.getenv("AWS_SECRET_ACCESS_KEY")
S3_BUCKET = os.getenv("S3_BUCKET", "keryx")
WHISPER_MODEL = os.getenv("WHISPER_MODEL", "medium")
WHISPER_MODEL = os.getenv("WHISPER_MODEL", "medium")
WHISPER_CACHE_DIR = os.getenv("WHISPER_CACHE_DIR")
WHISPER_CACHE_DIR = os.getenv("WHISPER_CACHE_DIR")


# ── S3 helpers ───────────────────────────────────────────────────────────────

s3_session = aioboto3.Session()


def _s3_client():
    return s3_session.client(
        "s3",
        endpoint_url=S3_ENDPOINT,
        aws_access_key_id=S3_ACCESS_KEY,
        aws_secret_access_key=S3_SECRET_KEY,
        verify=False,
    )


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
        headers = {"User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/121.0.0.0 Safari/537.36"}
        async with httpx.AsyncClient(verify=False, follow_redirects=True, headers=headers) as client:
            async with client.stream("GET", url) as resp:
                resp.raise_for_status()
                with open(dest, "wb") as f:
                    async for chunk in resp.aiter_bytes(8192):
                        f.write(chunk)
        
        if not os.path.exists(dest) or os.path.getsize(dest) == 0:
            raise HTTPException(500, f"Downloaded file from {url} is empty or missing.")


# ── API Models ───────────────────────────────────────────────────────────────

class TranscribeRequest(BaseModel):
    audio_url: str
    job_id: str
    language: Optional[str] = None  # None = auto-detect


class Segment(BaseModel):
    start: float
    end: float
    text: str
    translated: Optional[str] = None


class TranscribeResponse(BaseModel):
    status: str
    segments: List[Segment]
    duration: float
    language: str
    service: str = SERVICE_NAME


# ── Endpoints ────────────────────────────────────────────────────────────────

@app.get("/health")
async def health():
    return {
        "status": "ok",
        "service": SERVICE_NAME,
        "version": "1.0.0",
        "whisper_model": WHISPER_MODEL,
    }


@app.post("/transcribe", response_model=TranscribeResponse)
async def transcribe(req: TranscribeRequest):
    request_id = str(uuid.uuid4())[:8]
    logger.info(f"[{request_id}] Transcribe job={req.job_id} lang={req.language} url={req.audio_url}")
    start_time = time.time()

    tmp_dir = tempfile.mkdtemp(prefix=f"keryx_ve_{request_id}_")
    try:
        # 1. Download audio
        audio_path = os.path.join(tmp_dir, "audio.wav")
        await download_file_s3(req.audio_url, audio_path)

        if not os.path.exists(audio_path) or os.path.getsize(audio_path) == 0:
            raise HTTPException(500, "Audio download failed or empty")

        # 2. Transcribe
        model = await get_whisper()
        logger.info(f"[{request_id}] Running Whisper transcription...")

        result = await asyncio.to_thread(
            model.transcribe,
            audio_path,
            language=req.language,
            verbose=False,
            word_timestamps=False,
        )

        raw_segments = result.get("segments", [])
        duration = raw_segments[-1]["end"] if raw_segments else 0.0
        detected_lang = result.get("language", req.language or "unknown")

        segments = [
            Segment(
                start=round(s["start"], 3),
                end=round(s["end"], 3),
                text=s["text"].strip(),
            )
            for s in raw_segments
        ]

        elapsed = time.time() - start_time
        logger.info(f"[{request_id}] {len(segments)} segments, {duration:.1f}s audio, done in {elapsed:.1f}s")

        return TranscribeResponse(
            status="success",
            segments=segments,
            duration=duration,
            language=detected_lang,
        )

    except HTTPException:
        raise
    except Exception as e:
        logger.exception(f"[{request_id}] Error")
        raise HTTPException(500, str(e))
    finally:
        shutil.rmtree(tmp_dir, ignore_errors=True)


if __name__ == "__main__":
    import uvicorn
    # Configure uvicorn log format to match basicConfig
    log_config = uvicorn.config.LOGGING_CONFIG
    log_fmt = "%(asctime)s [%(levelname)s] %(name)s: %(message)s"
    log_config["formatters"]["access"]["fmt"] = log_fmt
    log_config["formatters"]["default"]["fmt"] = log_fmt
    
    # Filter out health check access logs from uvicorn
    logging.getLogger("uvicorn.access").addFilter(HealthCheckFilter())
    port = int(os.getenv("PORT", "8000"))
    uvicorn.run(app, host="0.0.0.0", port=port, log_config=log_config)
