# syntax=docker/dockerfile:1
FROM pytorch/pytorch:2.3.1-cuda12.1-cudnn8-runtime

# Prevent Python from writing .pyc files & buffer stdout/stderr
ENV PYTHONDONTWRITEBYTECODE=1
ENV PYTHONUNBUFFERED=1

# HuggingFace cache directory (mapped to a volume in docker-compose)
ENV HF_HOME=/app/huggingface

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

# 3. Install dependencies from requirements.txt
COPY requirements.txt .
RUN --mount=type=cache,target=/root/.cache/uv \
    uv pip install -r requirements.txt

# 4. Final step: Copy only the application code
COPY main.py .

# Create non-root user and cache directories
RUN useradd -m keryx && \
    mkdir -p $HF_HOME && \
    chown -R keryx:keryx /app

USER keryx

EXPOSE 8000
CMD ["uvicorn", "main:app", "--host", "0.0.0.0", "--port", "8000"]
