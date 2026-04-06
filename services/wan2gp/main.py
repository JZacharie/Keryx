import os
import io
import uuid
import time
import logging
import torch
import tempfile
import numpy as np
from typing import Optional
from fastapi import FastAPI, HTTPException
from pydantic import BaseModel
from diffusers import StableVideoDiffusionPipeline
from PIL import Image
import boto3
from urllib.parse import urlparse
from moviepy.editor import ImageSequenceClip
from huggingface_hub import login as hf_login

# Configure Verbose Logging
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s [%(levelname)s] %(name)s: %(message)s',
    datefmt='%Y-%m-%d %H:%M:%S'
)
logger = logging.getLogger("keryx.wan2gp")

app = FastAPI(title="Keryx Wan2GP Animation Engine")

# Configuration
S3_ENDPOINT = os.getenv("S3_ENDPOINT", "https://minio-170-api.zacharie.org")
S3_ACCESS_KEY = os.getenv("AWS_ACCESS_KEY_ID")
S3_SECRET_KEY = os.getenv("AWS_SECRET_ACCESS_KEY")
S3_BUCKET = os.getenv("S3_BUCKET", "keryx")
MODEL_ID = os.getenv("MODEL_ID", "stabilityai/stable-video-diffusion-img2vid-xt")
HF_TOKEN = os.getenv("HF_TOKEN")
DEVICE = "cuda" if torch.cuda.is_available() else "cpu"

if HF_TOKEN:
    hf_login(token=HF_TOKEN)

print(f"Loading Animation Pipeline on {DEVICE}...")
torch_dtype = torch.float16 if DEVICE == "cuda" else torch.float32

# Load Video Pipeline
pipe = StableVideoDiffusionPipeline.from_pretrained(
    MODEL_ID,
    torch_dtype=torch_dtype,
    variant="fp16" if DEVICE == "cuda" else None,
    token=HF_TOKEN
)

if DEVICE == "cuda":
    pipe.enable_model_cpu_offload()
    pipe.enable_attention_slicing()
else:
    pipe.to(DEVICE)

print("Animation Pipeline loaded successfully.")

s3_client = boto3.client(
    "s3",
    endpoint_url=S3_ENDPOINT,
    aws_access_key_id=S3_ACCESS_KEY,
    aws_secret_access_key=S3_SECRET_KEY,
    verify=False
)

class AnimationRequest(BaseModel):
    image_url: str
    fps: int = 14
    motion_bucket_id: int = 127
    noise_aug_strength: float = 0.02
    target_path: Optional[str] = None
    num_frames: int = 25 # Standard SVD-XT frame count

@app.get("/health")
def health():
    return {"status": "ok", "device": DEVICE, "model": MODEL_ID}

def download_image(url: str) -> Image.Image:
    parsed = urlparse(url)
    if any(host in parsed.netloc for host in ["zacharie.org", "minio", "localhost"]):
        parts = parsed.path.lstrip("/").split("/")
        bucket = parts[0]
        key = "/".join(parts[1:])
        response = s3_client.get_object(Bucket=bucket, Key=key)
        return Image.open(io.BytesIO(response["Body"].read())).convert("RGB")
    else:
        import requests
        response = requests.get(url)
        return Image.open(io.BytesIO(response.content)).convert("RGB")

def upload_video(video_bytes: io.BytesIO, key: str) -> str:
    video_bytes.seek(0)
    s3_client.put_object(
        Bucket=S3_BUCKET,
        Key=key,
        Body=video_bytes,
        ContentType="video/mp4"
    )
    return f"{S3_ENDPOINT}/{S3_BUCKET}/{key}"

def create_video_moviepy(frames: list, fps: int) -> io.BytesIO:
    """Assembles frames into a seamless ping-pong loop MP4."""
    # Convert PIL frames to numpy arrays
    clip_frames = [np.array(frame) for frame in frames]

    # Create Ping-Pong loop for seamless visual transition back to start
    # [1, 2, 3, 2] instead of [1, 2, 3, 1, 2, 3]
    pingpong = clip_frames + clip_frames[::-1][1:-1]

    clip = ImageSequenceClip(pingpong, fps=fps)
    buffer = io.BytesIO()

    with tempfile.NamedTemporaryFile(suffix=".mp4") as tmp:
        clip.write_videofile(tmp.name, codec="libx264", audio=False, logger=None, threads=4)
        with open(tmp.name, "rb") as f:
            buffer.write(f.read())

    return buffer

@app.post("/animate")
async def animate_image(request: AnimationRequest):
    request_id = str(uuid.uuid4())[:8]
    logger.info(f"[{request_id}] Received animation request for: {request.image_url}")
    start_time = time.time()

    try:
        # 1. Download and Prepare
        init_image = download_image(request.image_url).resize((1024, 576)) # Standard SVD-XT aspect ratio

        # 2. Run Inference
        logger.info(f"[{request_id}] Starting Animation generation (Frames: {request.num_frames})...")
        generator = torch.manual_seed(int(time.time()))
        with torch.inference_mode():
            # Generate frames
            output = pipe(
                init_image,
                decode_chunk_size=8,
                generator=generator,
                motion_bucket_id=request.motion_bucket_id,
                noise_aug_strength=request.noise_aug_strength,
                num_frames=request.num_frames
            )
            frames = output.frames[0]

        # 3. Assemble into Video Loop
        logger.info(f"[{request_id}] Assembling ping-pong loop video (Total frames: {len(frames)*2 - 2})...")
        video_buffer = create_video_moviepy(frames, request.fps)

        # 4. Upload result
        if not request.target_path:
            filename = f"animation_{uuid.uuid4()}.mp4"
            target_key = f"animations/{filename}"
        else:
            target_key = request.target_path

        logger.info(f"[{request_id}] Uploading result to S3: {target_key}")
        result_url = upload_video(video_buffer, target_key)

        duration = time.time() - start_time
        logger.info(f"[{request_id}] Animation request finished in {duration:.2f}s. Result: {result_url}")

        return {
            "status": "success",
            "url": result_url,
            "duration_est": f"{(len(frames)*2 - 2)/request.fps:.2f}s",
            "frames": len(frames)
        }

    except Exception as e:
        logger.error(f"[{request_id}] Error during animation: {str(e)}", exc_info=True)
        raise HTTPException(status_code=500, detail=str(e))

if __name__ == "__main__":
    import uvicorn
    uvicorn.run(app, host="0.0.0.0", port=8001)
