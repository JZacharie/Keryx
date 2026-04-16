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

# 1. Fix Ubuntu mirrors and install system dependencies
RUN sed -i 's/archive.ubuntu.com/mirrors.kernel.org/g' /etc/apt/sources.list && \
    sed -i 's/security.ubuntu.com/mirrors.kernel.org/g' /etc/apt/sources.list && \
    apt-get update && apt-get install -y --no-install-recommends \
    libgl1 \
    libglib2.0-0 \
    ffmpeg \
    git \
    curl \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# 2. Install uv for fast dependency management
ENV UV_SYSTEM_PYTHON=1
RUN curl -LsSf https://astral.sh/uv/install.sh | sh && \
    mv /root/.local/bin/uv /root/.local/bin/uvx /bin/

# 3. Install heavy dependencies first to maximize cache hits
RUN uv pip install setuptools wheel pip
RUN --mount=type=cache,target=/root/.cache/uv \
    uv pip install torch transformers diffusers accelerate
COPY requirements.txt .
RUN --mount=type=cache,target=/root/.cache/uv \
    uv pip install -r requirements.txt --no-build-isolation

# 5. Final step: Copy only the application code
COPY main.py .

# Create cache directories and set permissions
RUN mkdir -p $HF_HOME $XDG_CACHE_HOME && \
    chown -R 1000:1000 /app

USER 1000

EXPOSE 8000

# Run with uvicorn
CMD ["uvicorn", "main:app", "--host", "0.0.0.0", "--port", "8000"]
