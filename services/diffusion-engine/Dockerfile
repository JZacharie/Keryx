# syntax=docker/dockerfile:1
FROM pytorch/pytorch:2.2.1-cuda12.1-cudnn8-runtime
ENV DEBIAN_FRONTEND=noninteractive


# Prevent Python from writing .pyc files & buffer stdout/stderr
ENV PYTHONDONTWRITEBYTECODE=1
ENV PYTHONUNBUFFERED=1

# HuggingFace & Whisper cache directory (mapped to a volume in docker-compose)
ENV HF_HOME=/app/.cache/huggingface
ENV XDG_CACHE_HOME=/app/.cache

WORKDIR /app

# 1. Install system dependencies (rarely change)
RUN --mount=type=cache,target=/var/cache/apt,sharing=locked \
    --mount=type=cache,target=/var/lib/apt/lists,sharing=locked \
    apt-get update && apt-get install -y --no-install-recommends \
    libgl1-mesa-glx \
    libglib2.0-0 \
    ffmpeg \
    git

# 2. Upgrade pip, setuptools and wheel + set up cache for pip
RUN --mount=type=cache,target=/root/.cache/pip \
    pip install --upgrade pip setuptools wheel

# 3. Handle Python dependencies
# Copy requirements first to leverage Docker layer caching
COPY requirements.txt .

# Install dependencies using the BuildKit cache mount
# Even if requirements.txt changes, it will reuse already downloaded wheels
RUN --mount=type=cache,target=/root/.cache/pip \
    pip install -r requirements.txt

# 4. Final step: Copy the application code
# This layer will be rebuilt quickly on every code change
COPY . .

# Ensure HF_HOME exists and is writeable
RUN mkdir -p $HF_HOME

EXPOSE 8000

# Run with uvicorn
CMD ["uvicorn", "main:app", "--host", "0.0.0.0", "--port", "8000"]
