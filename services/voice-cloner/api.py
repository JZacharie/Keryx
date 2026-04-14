import os
import io
import uuid
import time
import logging
import tempfile
import shutil
import asyncio
from typing import Optional

import torch
from fastapi import FastAPI, HTTPException, Response
from pydantic import BaseModel
import aioboto3
import httpx
from urllib.parse import urlparse
from TTS.api import TTS

logging.basicConfig(
    level=logging.DEBUG,
    format="%(asctime)s [%(levelname)s] %(name)s: %(message)s",
    datefmt="%Y-%m-%d %H:%M:%S",
)
logger = logging.getLogger("keryx.voice_cloner")

app = FastAPI(title="Keryx Voice Cloner (XTTS v2)", version="1.0.0")

# Force agreement to Coqui non-commercial license
os.environ["COQUI_TOS_AGREED"] = "1"

# Configuration S3
S3_ENDPOINT = os.getenv("S3_ENDPOINT", "https://minio-170-api.zacharie.org")
S3_ACCESS_KEY = os.getenv("S3_ACCESS_KEY_ID")
S3_SECRET_KEY = os.getenv("S3_SECRET_ACCESS_KEY")
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

# Initialize TTS Model
device = "cuda" if torch.cuda.is_available() else "cpu"
logger.info(f"Loading XTTS v2 on {device}...")
tts = TTS("tts_models/multilingual/multi-dataset/xtts_v2").to(device)
logger.info("Model loaded.")

class CloneRequest(BaseModel):
    text: str
    language: str = "en"
    reference_url: str  # URL S3 ou HTTP du WAV de référence
    job_id: str
    output_key: Optional[str] = None

async def download_file(url: str, dest: str):
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

@app.get("/health")
def health():
    return {"status": "ok", "device": device, "model": "xtts_v2"}

@app.post("/clone")
async def clone_voice(req: CloneRequest):
    request_id = str(uuid.uuid4())[:8]
    logger.info(f"[{request_id}] Clone request job={req.job_id} lang={req.language}")
    start_time = time.time()

    tmp_dir = tempfile.mkdtemp(prefix=f"keryx_tts_{request_id}_")
    try:
        # 1. Download reference wav
        ref_path = os.path.join(tmp_dir, "reference.wav")
        logger.info(f"[{request_id}] Downloading reference from {req.reference_url}")
        await download_file(req.reference_url, ref_path)

        # 2. Generate TTS
        out_path = os.path.join(tmp_dir, "output.wav")
        logger.info(f"[{request_id}] Generating TTS...")
        
        # TTS logic is often CPU/GPU intensive and synchronous in Coqui
        await asyncio.to_thread(
            tts.tts_to_file,
            text=req.text,
            speaker_wav=ref_path,
            language=req.language,
            file_path=out_path
        )

        # 3. Upload to S3
        key = req.output_key or f"jobs/{req.job_id}/audio/clone_{uuid.uuid4()}.wav"
        async with _s3_client() as s3:
            await s3.upload_file(out_path, S3_BUCKET, key, ExtraArgs={"ContentType": "audio/wav"})
        
        result_url = f"{S3_ENDPOINT}/{S3_BUCKET}/{key}"
        
        duration = time.time() - start_time
        logger.info(f"[{request_id}] Done in {duration:.1f}s → {result_url}")
        
        return {
            "status": "success",
            "url": result_url,
            "duration": f"{duration:.2f}s"
        }

    except Exception as e:
        logger.exception(f"[{request_id}] Error")
        raise HTTPException(status_code=500, detail=str(e))
    finally:
        shutil.rmtree(tmp_dir, ignore_errors=True)

# Garder l'ancien endpoint pour compatibilité temporaire si nécessaire
@app.get("/")
async def legacy_tts(text: str, language: str = "en", speaker_wav: str = "reference.wav"):
    # ... (implémentation simplifiée ou redirection)
    return {"error": "Please use POST /clone"}

if __name__ == "__main__":
    import uvicorn
    uvicorn.run(app, host="0.0.0.0", port=8000)
