# 🎥 Keryx - Wan2GP Animation Microservice

The **Wan2GP** microservice is responsible for generating high-quality looped animations from static frames extracted during the **Keryx** pipeline.

## 🚀 Features
- **I2V (Image-to-Video)**: Converts any source image into a 1-2 second animation.
- **Ping-Pong Looping**: Automatically generates a seamless loop by mirroring the generated frames (e.g., [1, 2, 3, 2]), ensuring visual continuity.
- **S3 Optimized**: Directly downloads from and uploads results to the Keryx MinIO storage.
- **VRAM Efficient**: Uses `model_cpu_offload` and `attention_slicing` to run on single-GPU nodes.

## 📡 API Usage

### 1. Generate Animation
`POST /animate`
```json
{
  "image_url": "https://minio.zacharie.org/keryx/cleaned/frame_0001.jpg",
  "fps": 14,
  "motion_bucket_id": 127
}
```

### 2. Health Monitoring
`GET /health`

## 🏗️ Technical Stack
- **AI Model**: Stable Video Diffusion (SVD-XT 1.1)
- **Framework**: FastAPI (Python 3.10+)
- **Video Processing**: MoviePy + FFmpeg (libx264)
- **Deployment**: GPU-enabled Docker image

## 🛠️ Configuration
Expected Environment Variables:
- `S3_ENDPOINT`: MinIO/S3 API URL.
- `AWS_ACCESS_KEY_ID`: Access key for S3.
- `AWS_SECRET_ACCESS_KEY`: Secret key for S3.
- `S3_BUCKET`: The destination bucket (default `keryx`).
- `MODEL_ID`: HuggingFace model ID (default `stabilityai/stable-video-diffusion-img2vid-xt-1-1`).
