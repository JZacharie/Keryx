"""
keryx-texts-translation — Traduction (Ollama ou deep-translator) + Raffinement de texte.

POST /translate   : Segments -> segments traduits
POST /refine      : Texte -> texte raffiné
"""
import os
import asyncio
import logging
import uuid
import time
from typing import Optional, List
import httpx
from fastapi import FastAPI, HTTPException
from pydantic import BaseModel

logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s [%(levelname)s] %(name)s: %(message)s",
    datefmt="%Y-%m-%d %H:%M:%S",
)
logger = logging.getLogger("keryx.texts_translation")

class HealthCheckFilter(logging.Filter):
    def filter(self, record: logging.LogRecord) -> bool:
        return "/health" not in record.getMessage()

app = FastAPI(title="Keryx Texts Translation", version="1.0.0")

SERVICE_NAME = "keryx-texts-translation"
OLLAMA_URL = os.getenv("OLLAMA_URL", "http://ollama.ollama.svc.cluster.local:11434")
OLLAMA_MODEL = os.getenv("OLLAMA_MODEL", "llama3")
# TRANSLATOR_BACKEND: "ollama" (default) ou "google" (fallback)
TRANSLATOR_BACKEND = os.getenv("TRANSLATOR_BACKEND", "ollama")

# -- Models --

class Segment(BaseModel):
    start: float
    end: float
    text: str
    translated: Optional[str] = None

class TranslateRequest(BaseModel):
    segments: List[Segment]
    target_lang: str
    job_id: str

class TranslateResponse(BaseModel):
    status: str
    segments: List[Segment]
    service: str = SERVICE_NAME

class RefineRequest(BaseModel):
    text: str
    job_id: str

class RefineResponse(BaseModel):
    status: str
    refined_text: str
    service: str = SERVICE_NAME

# -- Logic --

async def translate_with_ollama(text: str, target_lang: str) -> str:
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

async def refine_text(text: str) -> str:
    if TRANSLATOR_BACKEND != "ollama":
        return text
    prompt = (
        "You are an expert editor. Below is a transcribed text composed of several sentence fragments. "
        "Rewrite this text to be fluid, coherent, and professional, while strictly maintaining the original language and meaning. "
        "Correct any speech-to-text errors and remove filler words. "
        "Return ONLY the refined text as a single paragraph:\n\n" + text
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
        logger.warning(f"Ollama refinement failed: {e}. Returning original.")
        return text

# -- Endpoints --

@app.get("/health")
async def health():
    return {
        "status": "ok",
        "service": SERVICE_NAME,
        "version": "1.0.0",
        "translator_backend": TRANSLATOR_BACKEND,
    }

@app.post("/translate", response_model=TranslateResponse)
async def translate(req: TranslateRequest):
    request_id = str(uuid.uuid4())[:8]
    logger.info(f"[{request_id}] Translate {len(req.segments)} segments -> {req.target_lang} (job={req.job_id})")
    start_time = time.time()
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

@app.post("/refine", response_model=RefineResponse)
async def refine(req: RefineRequest):
    request_id = str(uuid.uuid4())[:8]
    logger.info(f"[{request_id}] Refining text for job={req.job_id}")
    start_time = time.time()
    refined = await refine_text(req.text)
    elapsed = time.time() - start_time
    logger.info(f"[{request_id}] Refinement done in {elapsed:.1f}s")
    return RefineResponse(status="success", refined_text=refined)

if __name__ == "__main__":
    import uvicorn
    port = int(os.getenv("PORT", "8000"))
    uvicorn.run(app, host="0.0.0.0", port=port)
