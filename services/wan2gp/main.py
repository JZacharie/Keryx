import os
import io
import uuid
import time
import logging
import asyncio
from typing import Optional

import torch
import tempfile
import numpy as np
from fastapi import FastAPI, HTTPException
from pydantic import BaseModel
from diffusers import StableVideoDiffusionPipeline
from PIL import Image
import aioboto3
from urllib.parse import urlparse
from moviepy.editor import ImageSequenceClip
import httpx

# Configure Logging
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s [%(levelname)s] %(name)s: %(message)s',
    datefmt='%Y-%m-%d %H:%M:%S'
)
logger = logging.getLogger("keryx.video_generator")

class HealthCheckFilter(logging.Filter):
    def filter(self, record: logging.LogRecord) -> bool:
        if "/health" in record.getMessage():
            record.levelno = logging.DEBUG
            record.levelname = "DEBUG"
        return True

app = FastAPI(title="Keryx Video Generator (SVD)", version="1.0.0")

# Configuration
SERVICE_NAME = "keryx-wan2gp"
S3_ENDPOINT = os.getenv("S3_ENDPOINT", "https://minio-170-api.zacharie.org")
S3_ACCESS_KEY = os.getenv("S3_ACCESS_KEY_ID") or os.getenv("AWS_ACCESS_KEY_ID")
S3_SECRET_KEY = os.getenv("S3_SECRET_ACCESS_KEY") or os.getenv("AWS_SECRET_ACCESS_KEY")
S3_BUCKET = os.getenv("S3_BUCKET", "keryx")
MODEL_ID = os.getenv("MODEL_ID", "stabilityai/stable-video-diffusion-img2vid-xt-1-1")
DEVICE = "cuda" if torch.cuda.is_available() else "cpu"

print(f"Loading SVD Pipeline on {DEVICE}...")
torch_dtype = torch.float16 if DEVICE == "cuda" else torch.float32

# Load Video Pipeline
pipe = StableVideoDiffusionPipeline.from_pretrained(
    MODEL_ID,
    torch_dtype=torch_dtype,
    variant="fp16" if DEVICE == "cuda" else None
)

if DEVICE == "cuda":
    pipe.enable_model_cpu_offload()
    pipe.enable_attention_slicing()
else:
    pipe.to(DEVICE)

print("SVD Pipeline loaded successfully.")

s3_session = aioboto3.Session()

def _s3_client():
    return s3_session.client(
        "s3",
        endpoint_url=S3_ENDPOINT,
        aws_access_key_id=S3_ACCESS_KEY,
        aws_secret_access_key=S3_SECRET_KEY,
        verify=False
    )

class AnimationRequest(BaseModel):
    image_url: str
    job_id: str
    fps: int = 14
    motion_bucket_id: int = 127
    noise_aug_strength: float = 0.02
    num_frames: int = 25
    output_key: Optional[str] = None

@app.get("/health")
def health():
    return {"status": "ok", "device": DEVICE, "model": MODEL_ID, "service": SERVICE_NAME}

async def download_image(url: str) -> Image.Image:
    parsed = urlparse(url)
    if url.startswith("/") and os.path.exists(url):
        return Image.open(url).convert("RGB")
    if any(host in parsed.netloc for host in ["zacharie.org", "minio", "localhost", "rustfs"]):
        parts = parsed.path.lstrip("/").split("/")
        bucket, key = parts[0], "/".join(parts[1:])
        async with _s3_client() as s3:
            resp = await s3.get_object(Bucket=bucket, Key=key)
            data = await resp["Body"].read()
        return Image.open(io.BytesIO(data)).convert("RGB")
    else:
        async with httpx.AsyncClient(verify=False) as client:
            resp = await client.get(url)
            return Image.open(io.BytesIO(resp.content)).convert("RGB")

async def upload_video(video_bytes: bytes, key: str) -> str:
    async with _s3_client() as s3:
        await s3.put_object(
            Bucket=S3_BUCKET,
            Key=key,
            Body=video_bytes,
            ContentType="video/mp4"
        )
    return f"{S3_ENDPOINT}/{S3_BUCKET}/{key}"

def create_video_bytes(frames: list, fps: int) -> bytes:
    """Assembles frames into a seamless ping-pong loop MP4."""
    clip_frames = [np.array(frame) for frame in frames]
    # Create Ping-Pong loop
    pingpong = clip_frames + clip_frames[::-1][1:-1]
    clip = ImageSequenceClip(pingpong, fps=fps)
    
    with tempfile.NamedTemporaryFile(suffix=".mp4", delete=False) as tmp:
        tmp_path = tmp.name
    
    try:
        clip.write_videofile(tmp_path, codec="libx264", audio=False, logger=None, threads=4)
        with open(tmp_path, "rb") as f:
            data = f.read()
        return data
    finally:
        if os.path.exists(tmp_path):
            os.remove(tmp_path)

@app.post("/animate")
async def animate_image(req: AnimationRequest):
    request_id = str(uuid.uuid4())[:8]
    logger.info(f"[{request_id}] Animation request job={req.job_id} url={req.image_url}")
    start_time = time.time()

    try:
        # 1. Download
        init_image = await download_image(req.image_url)
        # SVD expects specific dimensions (usually 1024x576 or 512x288)
        init_image = init_image.resize((1024, 576))

        # 2. Inference
        logger.info(f"[{request_id}] Starting SVD generation...")
        generator = torch.manual_seed(int(time.time()))
        
        def _run_svd():
            with torch.inference_mode():
                return pipe(
                    init_image,
                    decode_chunk_size=8,
                    generator=generator,
                    motion_bucket_id=req.motion_bucket_id,
                    noise_aug_strength=req.noise_aug_strength,
                    num_frames=req.num_frames
                ).frames[0]

        frames = await asyncio.to_thread(_run_svd)

        # 3. Assemble
        logger.info(f"[{request_id}] Assembling video loop...")
        video_data = await asyncio.to_thread(create_video_bytes, frames, req.fps)

        # 4. Upload
        key = req.output_key or f"jobs/{req.job_id}/animations/anim_{uuid.uuid4()}.mp4"
        result_url = await upload_video(video_data, key)

        duration = time.time() - start_time
        logger.info(f"[{request_id}] Done in {duration:.1f}s → {result_url}")

        return {
            "status": "success",
            "url": result_url,
            "duration": f"{duration:.2f}s",
            "frames": len(frames)
        }

    except Exception as e:
        logger.exception(f"[{request_id}] Error")
        raise HTTPException(status_code=500, detail=str(e))

if __name__ == "__main__":
    import uvicorn
    # Filter out health check access logs from uvicorn
    logging.getLogger("uvicorn.access").addFilter(HealthCheckFilter())
    uvicorn.run(app, host="0.0.0.0", port=8000, log_level="info")
