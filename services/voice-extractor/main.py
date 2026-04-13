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
    level=logging.DEBUG,
    format="%(asctime)s [%(levelname)s] %(name)s: %(message)s",
    datefmt="%Y-%m-%d %H:%M:%S",
)
logger = logging.getLogger("keryx.voice_extractor")

app = FastAPI(title="Keryx Voice Extractor", version="1.0.0")

SERVICE_NAME = "keryx-voice-extractor"
S3_ENDPOINT = os.getenv("S3_ENDPOINT", "https://minio-170-api.zacharie.org")
S3_ACCESS_KEY = os.getenv("AWS_ACCESS_KEY_ID")
S3_SECRET_KEY = os.getenv("AWS_SECRET_ACCESS_KEY")
S3_BUCKET = os.getenv("S3_BUCKET", "keryx")
WHISPER_MODEL = os.getenv("WHISPER_MODEL", "medium")
OLLAMA_URL = os.getenv("OLLAMA_URL", "http://ollama.ollama.svc.cluster.local:11434")
OLLAMA_MODEL = os.getenv("OLLAMA_MODEL", "llama3")
# TRANSLATOR_BACKEND: "ollama" (default — meilleure qualité) ou "google" (fallback gratuit)
TRANSLATOR_BACKEND = os.getenv("TRANSLATOR_BACKEND", "ollama")

# ── Whisper — chargement lazy ────────────────────────────────────────────────
_whisper_model = None
_whisper_lock = asyncio.Lock()


async def get_whisper():
    global _whisper_model
    async with _whisper_lock:
        if _whisper_model is None:
            import torch
            import whisper
            device = "cuda" if torch.cuda.is_available() else "cpu"
            logger.info(f"Loading Whisper model '{WHISPER_MODEL}' on {device}...")
            _whisper_model = await asyncio.to_thread(
                whisper.load_model, WHISPER_MODEL, device=device
            )
            logger.info("Whisper model loaded.")
    return _whisper_model


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
        async with httpx.AsyncClient(verify=False) as client:
            async with client.stream("GET", url) as resp:
                with open(dest, "wb") as f:
                    async for chunk in resp.aiter_bytes(8192):
                        f.write(chunk)


# ── Traduction ───────────────────────────────────────────────────────────────

async def translate_with_ollama(text: str, target_lang: str) -> str:
    """Traduction via Ollama (meilleure qualité)."""
    lang_names = {
        "fr": "French", "es": "Spanish", "pt": "Portuguese",
        "de": "German", "it": "Italian", "zh": "Chinese", "ja": "Japanese",
        "ar": "Arabic", "en": "English",
    }
    lang_name = lang_names.get(target_lang, target_lang)
    prompt = (
        f"Translate the following text to {lang_name}. "
        f"Return ONLY the translation, no explanation, no quotes:\n{text}"
    )
    try:
        async with httpx.AsyncClient(timeout=60, verify=False) as client:
            resp = await client.post(
                f"{OLLAMA_URL}/api/generate",
                json={"model": OLLAMA_MODEL, "prompt": prompt, "stream": False},
            )
            resp.raise_for_status()
            return resp.json().get("response", text).strip()
    except Exception as e:
        logger.warning(f"Ollama translation failed: {e}. Falling back to Google.")
        return await translate_with_google(text, target_lang)


async def translate_with_google(text: str, target_lang: str) -> str:
    """Traduction via deep-translator (fallback)."""
    try:
        from deep_translator import GoogleTranslator
        translated = await asyncio.to_thread(
            GoogleTranslator(source="auto", target=target_lang).translate, text
        )
        return translated or text
    except Exception as e:
        logger.warning(f"Google translation failed: {e}. Returning original.")
        return text


async def translate_text(text: str, target_lang: str) -> str:
    if TRANSLATOR_BACKEND == "ollama":
        return await translate_with_ollama(text, target_lang)
    return await translate_with_google(text, target_lang)


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


class TranslateRequest(BaseModel):
    segments: List[Segment]
    target_lang: str
    job_id: str


class TranslateResponse(BaseModel):
    status: str
    segments: List[Segment]
    service: str = SERVICE_NAME


# ── Endpoints ────────────────────────────────────────────────────────────────

@app.get("/health")
async def health():
    return {
        "status": "ok",
        "service": SERVICE_NAME,
        "version": "1.0.0",
        "whisper_model": WHISPER_MODEL,
        "translator_backend": TRANSLATOR_BACKEND,
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


@app.post("/translate", response_model=TranslateResponse)
async def translate(req: TranslateRequest):
    request_id = str(uuid.uuid4())[:8]
    logger.info(f"[{request_id}] Translate {len(req.segments)} segments → {req.target_lang} (job={req.job_id})")
    start_time = time.time()

    # Traduire tous les segments en parallèle (max 5 concurrents)
    semaphore = asyncio.Semaphore(5)

    async def translate_segment(seg: Segment) -> Segment:
        async with semaphore:
            translated = await translate_text(seg.text, req.target_lang)
            return Segment(
                start=seg.start,
                end=seg.end,
                text=seg.text,
                translated=translated,
            )

    translated_segments = await asyncio.gather(*[translate_segment(s) for s in req.segments])

    elapsed = time.time() - start_time
    logger.info(f"[{request_id}] Translation done in {elapsed:.1f}s")

    return TranslateResponse(status="success", segments=list(translated_segments))


if __name__ == "__main__":
    import uvicorn
    uvicorn.run(app, host="0.0.0.0", port=8000)
