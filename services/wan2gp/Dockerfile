# syntax=docker/dockerfile:1
FROM pytorch/pytorch:2.3.1-cuda12.1-cudnn8-runtime

# Prevent Python from writing .pyc files & buffer stdout/stderr
ENV PYTHONDONTWRITEBYTECODE=1
ENV PYTHONUNBUFFERED=1

# HuggingFace cache directory (mapped to a volume in docker-compose)
ENV HF_HOME=/app/huggingface

WORKDIR /app

# 1. Install system dependencies (rarely change)
RUN apt-get update && apt-get install -y --no-install-recommends \
    libgl1 \
    libglib2.0-0 \
    ffmpeg \
    git \
    && rm -rf /var/lib/apt/lists/*

# 2. Install uv for fast dependency management
ENV UV_SYSTEM_PYTHON=1
COPY --from=ghcr.io/astral-sh/uv:latest /uv /uvx /bin/

# 3. Install heavy AI dependencies first
RUN uv pip install torch transformers diffusers accelerate

# 4. Install remaining app dependencies
COPY requirements.txt .
RUN uv pip install -r requirements.txt

# 4. Final step: Copy only the application code
COPY main.py .

# Add non-root user
RUN useradd -m keryx && chown -R keryx:keryx /app
USER keryx

EXPOSE 8000
CMD ["uvicorn", "main:app", "--host", "0.0.0.0", "--port", "8000"]
