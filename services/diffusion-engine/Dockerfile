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
RUN apt-get update && apt-get install -y --no-install-recommends \
    libgl1 \
    libglib2.0-0 \
    ffmpeg \
    git \
    && rm -rf /var/lib/apt/lists/*

# 2. Upgrade pip and install uv for fast dependency management
ENV UV_SYSTEM_PYTHON=1
COPY --from=ghcr.io/astral-sh/uv:latest /uv /uvx /bin/

# 3. Install heavy dependencies first to maximize cache hits
RUN uv pip install torch transformers diffusers accelerate

# 4. Install remaining app dependencies
COPY requirements.txt .
RUN uv pip install -r requirements.txt

# 5. Final step: Copy only the application code
COPY main.py .

# Create cache directories and set permissions
RUN mkdir -p $HF_HOME $XDG_CACHE_HOME && \
    chown -R 1000:1000 /app

USER 1000

EXPOSE 8000

# Run with uvicorn
CMD ["uvicorn", "main:app", "--host", "0.0.0.0", "--port", "8000"]
