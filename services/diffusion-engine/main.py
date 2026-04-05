import os
import io
import uuid
import uuid as uuid_pkg
import time
import numpy as np
import cv2
import logging
import subprocess
import tempfile
import shutil
import pathlib
from typing import Optional
from fastapi import FastAPI, HTTPException, BackgroundTasks
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
import boto3
from urllib.parse import urlparse

# Configure Verbose Logging
logging.basicConfig(
    level=logging.DEBUG,
    format='%(asctime)s [%(levelname)s] %(name)s: %(message)s',
    datefmt='%Y-%m-%d %H:%M:%S'
)
logger = logging.getLogger("keryx.diffusion")

app = FastAPI(title="Keryx Diffusion Engine")

# Configuration
S3_ENDPOINT = os.getenv("S3_ENDPOINT", "https://minio-170-api.zacharie.org")
S3_ACCESS_KEY = os.getenv("AWS_ACCESS_KEY_ID")
S3_SECRET_KEY = os.getenv("AWS_SECRET_ACCESS_KEY")
S3_BUCKET = os.getenv("S3_BUCKET", "keryx")
MODEL_ID = os.getenv("MODEL_ID", "stabilityai/sdxl-turbo")
CONTROLNET_ID = "diffusers/controlnet-canny-sdxl-1.0"
DEVICE = "cuda" if torch.cuda.is_available() else "cpu"

# Brand Colors (Teamwork.com)
TW_PINK = "#FF22B1"
TW_SLATE = "#1D1C39"
TW_WHITE = "#FFFFFF"

print(f"Loading models on {DEVICE}...")
torch_dtype = torch.float16 if DEVICE == "cuda" else torch.float32

# Load ControlNet
print(f"Loading ControlNet: {CONTROLNET_ID}")
controlnet = ControlNetModel.from_pretrained(
    CONTROLNET_ID,
    torch_dtype=torch_dtype,
    use_safetensors=True
)

# Load Pipeline
# Note: SDXL Turbo can be used with SDXL pipelines
print(f"Loading Pipeline: {MODEL_ID}")
pipe = StableDiffusionXLControlNetImg2ImgPipeline.from_pretrained(
    MODEL_ID,
    controlnet=controlnet,
    torch_dtype=torch_dtype,
    variant="fp16" if DEVICE == "cuda" else None,
    use_safetensors=True
)

if DEVICE == "cuda":
    pipe.enable_attention_slicing()
    pipe.enable_model_cpu_offload() # Handles moving to GPU automatically
else:
    pipe.to(DEVICE)
print("Models loaded successfully.")

s3_client = boto3.client(
    "s3",
    endpoint_url=S3_ENDPOINT,
    aws_access_key_id=S3_ACCESS_KEY,
    aws_secret_access_key=S3_SECRET_KEY,
    verify=False # Common for local MinIO
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
    return {"status": "ok", "device": DEVICE, "model": MODEL_ID, "controlnet": CONTROLNET_ID}

def download_image(url: str) -> Image.Image:
    parsed = urlparse(url)
    if "zacharie.org" in parsed.netloc or "minio" in parsed.netloc:
        parts = parsed.path.lstrip("/").split("/")
        bucket = parts[0]
        key = "/".join(parts[1:])
        response = s3_client.get_object(Bucket=bucket, Key=key)
        return Image.open(io.BytesIO(response["Body"].read())).convert("RGB")
    else:
        import requests
        response = requests.get(url)
        return Image.open(io.BytesIO(response.content)).convert("RGB")

def upload_image(image: Image.Image, key: str) -> str:
    buffer = io.BytesIO()
    image.save(buffer, format="JPEG", quality=90)
    buffer.seek(0)
    s3_client.put_object(
        Bucket=S3_BUCKET,
        Key=key,
        Body=buffer,
        ContentType="image/jpeg"
    )
    return f"{S3_ENDPOINT}/{S3_BUCKET}/{key}"

def download_video(url: str, dest_path: str):
    parsed = urlparse(url)
    if any(h in parsed.netloc for h in ["zacharie.org", "minio", "localhost"]):
        parts = parsed.path.lstrip("/").split("/")
        bucket = parts[0]
        key = "/".join(parts[1:])
        s3_client.download_file(bucket, key, dest_path)
    else:
        import requests
        response = requests.get(url, stream=True)
        with open(dest_path, "wb") as f:
            for chunk in response.iter_content(chunk_size=8192):
                f.write(chunk)

def upload_video(path: str, key: str) -> str:
    s3_client.upload_file(
        path,
        S3_BUCKET,
        key,
        ExtraArgs={"ContentType": "video/mp4"}
    )
    return f"{S3_ENDPOINT}/{S3_BUCKET}/{key}"

def get_canny_image(image: Image.Image) -> Image.Image:
    image_np = np.array(image)
    image_np = cv2.Canny(image_np, 100, 200)
    image_np = image_np[:, :, None]
    image_np = np.concatenate([image_np, image_np, image_np], axis=2)
    return Image.fromarray(image_np)

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
        init_image = download_image(request.image_url)
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
            target_key = f"styled/{filename}"
        else:
            target_key = request.target_path

        logger.info(f"[{request_id}] Uploading result to S3: {target_key}")
        result_url = upload_image(stylized_image, target_key)

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
        # 1. Download
        init_image = download_image(request.image_url)
        w, h = init_image.size

        # 2. Create NotebookLM Mask (bottom right)
        mask = Image.new("L", (w, h), 0)
        from PIL import ImageDraw, ImageFilter
        draw = ImageDraw.Draw(mask)
        # NotebookLM zone: [x_start, y_start, x_end, y_end]
        draw.rectangle([w * 0.82, h * 0.90, w, h], fill=255)
        mask = mask.filter(ImageFilter.GaussianBlur(radius=5))

        # 3. Setup Inpaint Pipeline (Regular SDXL Inpaint, no ControlNet needed for this)
        # We use from_pipe to share components and save VRAM
        inpaint_pipe = StableDiffusionXLInpaintPipeline.from_pipe(pipe)
        if DEVICE != "cuda":
            inpaint_pipe.to(DEVICE)
        else:
            inpaint_pipe.enable_model_cpu_offload()

        # 4. Run Inference
        logger.info(f"[{request_id}] Starting Inpaint for watermark cleaning...")
        with torch.inference_mode():
            images = inpaint_pipe(
                prompt="matching background texture, seamless, clean, white background",
                negative_prompt="text, logo, blurry, distorted, watermark",
                image=init_image,
                mask_image=mask,
                num_inference_steps=20,
                strength=1.0
            ).images

        cleaned_image = images[0]

        # 5. Upload result
        if not request.target_path:
            filename = f"cleaned_{uuid_pkg.uuid4()}.jpg"
            target_key = f"cleaned/{filename}"
        else:
            target_key = request.target_path

        logger.info(f"[{request_id}] Uploading result to S3: {target_key}")
        result_url = upload_image(cleaned_image, target_key)

        duration = time.time() - start_time
        logger.info(f"[{request_id}] Request finished in {duration:.2f}s. Result: {result_url}")

        return {
            "status": "success",
            "url": result_url,
            "target": target_key
        }

    except Exception as e:
        logger.error(f"[{request_id}] Error during watermark cleaning: {str(e)}", exc_info=True)
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
        download_video(request.video_url, video_path)

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

        # 3. STAGE 2: Upload ALL Raw Frames to S3 (Persist raw data before processing)
        logger.info(f"[{request_id}] Uploading {frame_count} raw frames to S3 (raw_frames/{request_id}/)...")
        for i in range(1, frame_count + 1):
            filename = f"frame_{i:04d}.jpg"
            local_path = f"{raw_frames_dir}/{filename}"
            s3_key = f"raw_frames/{request_id}/{filename}"
            s3_client.upload_file(local_path, S3_BUCKET, s3_key)
            if i % 100 == 0:
                logger.debug(f"[{request_id}] Uploaded {i}/{frame_count} raw frames...")
        logger.info(f"[{request_id}] All raw frames successfully pushed to S3.")

        # 4. Setup Inpaint Pipeline (After extraction and upload are confirmed)
        logger.info(f"[{request_id}] Phase 3: Initializing SDXL Inpaint pipeline...")
        # Manually sharing components with explicit float16 to avoid dtype mismatch
        inpaint_pipe = StableDiffusionXLInpaintPipeline(
            vae=pipe.vae,
            text_encoder=pipe.text_encoder,
            text_encoder_2=pipe.text_encoder_2,
            tokenizer=pipe.tokenizer,
            tokenizer_2=pipe.tokenizer_2,
            unet=pipe.unet,
            scheduler=pipe.scheduler,
            force_zeros_for_empty_prompt=True,
            add_watermarker=False
        ).to(DEVICE, torch.float16)

        logger.info(f"[{request_id}] Pipeline ready (Dtype: {inpaint_pipe.dtype}, Device: {inpaint_pipe.device}). Starting watermark removal...")

        # 5. Process each frame (Downloading back from S3 as requested)
        from PIL import ImageDraw, ImageFilter
        for i in range(1, frame_count + 1):
            filename = f"frame_{i:04d}.jpg"
            s3_key = f"raw_frames/{request_id}/{filename}"
            local_raw_path = f"{tmp_dir}/s3_downloaded_{filename}"

            # Pull from S3
            s3_client.download_file(S3_BUCKET, s3_key, local_raw_path)

            if i % 10 == 0 or i == 1:
                logger.info(f"[{request_id}] Processing frame {i}/{frame_count} (Downloaded from S3)...")

            # Load frame and prepare
            init_image = Image.open(local_raw_path).convert("RGB")
            w, h = init_image.size

            # Create Mask
            mask = Image.new("L", (w, h), 0)
            draw = ImageDraw.Draw(mask)
            draw.rectangle([w * 0.82, h * 0.90, w, h], fill=255)
            mask = mask.filter(ImageFilter.GaussianBlur(radius=5))

            # Inpaint
            with torch.inference_mode():
                result = inpaint_pipe(
                    prompt="matching background texture, seamless, clean, white background",
                    negative_prompt="text, logo, blurry, distorted, watermark",
                    image=init_image,
                    mask_image=mask,
                    num_inference_steps=15,
                    strength=1.0
                ).images[0]

            # Save cleaned frame
            frame_output_filename = f"{cleaned_dir}/frame_{i:04d}.jpg"
            result.save(frame_output_filename, quality=90)

            # Cleanup downloaded raw frame to save disk
            if os.path.exists(local_raw_path):
                os.remove(local_raw_path)

        # 6. Assemble Video using FFmpeg
        output_video = f"{tmp_dir}/output.mp4"
        logger.info(f"[{request_id}] Re-assembling video with FFmpeg...")
        assemble_cmd = [
            "ffmpeg", "-y", "-framerate", str(fps),
            "-i", f"{cleaned_dir}/frame_%04d.jpg",
            "-c:v", "libx264", "-pix_fmt", "yuv420p",
            "-crf", "23", # Good balance of quality/size
            output_video
        ]
        subprocess.run(assemble_cmd, check=True, capture_output=True)

        # 7. Upload Final Result
        if not request.target_path:
            final_filename = f"video_cleaned_{uuid_pkg.uuid4()}.mp4"
            target_key = f"cleaned_videos/{final_filename}"
        else:
            target_key = request.target_path

        logger.info(f"[{request_id}] Uploading final video to S3: {target_key}")
        result_url = upload_video(output_video, target_key)

        duration = time.time() - start_time
        logger.info(f"[{request_id}] Video cleaning finished in {duration:.2f}s. Result: {result_url}")

        return {
            "status": "success",
            "url": result_url,
            "frames_processed": frame_count,
            "duration": f"{duration:.2f}s",
            "raw_frames_s3": f"raw_frames/{request_id}/"
        }

    except Exception as e:
        logger.error(f"[{request_id}] Error during video cleaning: {str(e)}", exc_info=True)
        raise HTTPException(status_code=500, detail=str(e))
    finally:
        # Cleanup
        shutil.rmtree(tmp_dir, ignore_errors=True)

@app.post("/inpaint")
async def inpaint_image(request: InpaintRequest):
    request_id = str(uuid_pkg.uuid4())[:8]
    logger.info(f"[{request_id}] Received inpaint request for: {request.image_url}")
    start_time = time.time()
    try:
        # 1. Download and Prepare
        init_image = download_image(request.image_url).resize((1024, 1024))
        mask_image = download_image(request.mask_url).resize((1024, 1024))

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
            target_key = f"inpainted/{filename}"
        else:
            target_key = request.target_path

        logger.info(f"[{request_id}] Uploading result to S3: {target_key}")
        result_url = upload_image(inpainted_image, target_key)

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
    uvicorn.run(app, host="0.0.0.0", port=8000)
