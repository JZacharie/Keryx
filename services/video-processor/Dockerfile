# syntax=docker/dockerfile:1
FROM pytorch/pytorch:2.2.1-cuda12.1-cudnn8-runtime
ENV DEBIAN_FRONTEND=noninteractive


# Prevent Python from writing .pyc files & buffer stdout/stderr
ENV PYTHONDONTWRITEBYTECODE=1
ENV PYTHONUNBUFFERED=1

# HuggingFace & Whisper cache directory (mapped to a volume in docker-compose)
ENV HF_HOME=/app/huggingface
ENV WHISPER_CACHE=/app/whisper_models

WORKDIR /app

# 1. Install system dependencies (rarely change)
RUN apt-get update && apt-get install -y --no-install-recommends \
    libgl1-mesa-glx \
    libglib2.0-0 \
    ffmpeg \
    git \
    libopencv-dev \
    python3-opencv \
    && rm -rf /var/lib/apt/lists/*

# 2. Upgrade pip and set up cache for pip
RUN --mount=type=cache,target=/root/.cache/pip \
    pip install --upgrade pip

# 3. Handle Python dependencies
COPY requirements.txt .

# Install dependencies using the BuildKit cache mount
RUN --mount=type=cache,target=/root/.cache/pip \
    pip install -r requirements.txt

# 4. Final step: Copy the application code
COPY main.py .

# Ensure cache directories exist
RUN mkdir -p $HF_HOME $WHISPER_CACHE /app/outputs

# Add command to run the processor
CMD ["python", "main.py"]
