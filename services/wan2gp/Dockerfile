# syntax=docker/dockerfile:1
FROM pytorch/pytorch:2.2.1-cuda12.1-cudnn8-runtime

# Prevent Python from writing .pyc files & buffer stdout/stderr
ENV PYTHONDONTWRITEBYTECODE=1
ENV PYTHONUNBUFFERED=1

# HuggingFace cache directory (mapped to a volume in docker-compose)
ENV HF_HOME=/app/huggingface

WORKDIR /app

# 1. Install system dependencies
# These change rarely, so they stay at the top
RUN apt-get update && apt-get install -y --no-install-recommends \
    libgl1-mesa-glx \
    libglib2.0-0 \
    ffmpeg \
    git \
    && rm -rf /var/lib/apt/lists/*

# 2. Upgrade pip and set up cache for pip
RUN --mount=type=cache,target=/root/.cache/pip \
    pip install --upgrade pip

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

EXPOSE 8001

# Run with uvicorn
CMD ["uvicorn", "main:app", "--host", "0.0.0.0", "--port", "8001"]
