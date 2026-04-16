# syntax=docker/dockerfile:1
FROM pytorch/pytorch:2.3.1-cuda12.1-cudnn8-runtime
ENV DEBIAN_FRONTEND=noninteractive


# Prevent Python from writing .pyc files & buffer stdout/stderr
ENV PYTHONDONTWRITEBYTECODE=1
ENV PYTHONUNBUFFERED=1

# HuggingFace & Whisper cache directory
ENV HF_HOME=/app/huggingface
ENV XDG_CACHE_HOME=/app/cache

WORKDIR /app

# 1. Install system dependencies (rarely change)
RUN --mount=type=cache,target=/var/cache/apt,sharing=locked \
    --mount=type=cache,target=/var/lib/apt/lists,sharing=locked \
    apt-get update && apt-get install -y --no-install-recommends \
    libgl1-mesa-glx \
    libglib2.0-0 \
    ffmpeg \
    git

# 2. Upgrade pip and install heavy AI dependencies first (these change rarely)
RUN --mount=type=cache,target=/root/.cache/pip \
    pip install --upgrade pip setuptools wheel && \
    pip install \
    torch==2.2.1 \
    diffusers==0.26.3 \
    transformers==4.38.2 \
    accelerate==0.27.2 \
    opencv-python-headless==4.9.0.80

# 3. Handle remaining Python dependencies
COPY requirements.txt .
RUN --mount=type=cache,target=/root/.cache/pip \
    pip install -r requirements.txt

# 4. Final step: Copy only the application code
COPY main.py .

# Create cache directories and set permissions
RUN mkdir -p $HF_HOME $XDG_CACHE_HOME && \
    chown -R 1000:1000 /app

USER 1000

EXPOSE 8000

# Run with uvicorn
CMD ["uvicorn", "main:app", "--host", "0.0.0.0", "--port", "8000"]
