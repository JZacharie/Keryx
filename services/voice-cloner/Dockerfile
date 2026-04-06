# Use PyTorch with CUDA support
FROM pytorch/pytorch:2.2.1-cuda12.1-cudnn8-runtime

WORKDIR /app

# Install system dependencies
RUN --mount=type=cache,target=/var/cache/apt,sharing=locked \
    --mount=type=cache,target=/var/lib/apt/lists,sharing=locked \
    apt-get update && apt-get install -y --no-install-recommends \
    git \
    ffmpeg \
    libsndfile1

# Install Coqui TTS and other dependencies with pip cache
RUN --mount=type=cache,target=/root/.cache/pip \
    pip install \
    TTS \
    fastapi \
    uvicorn \
    python-multipart

# Create directory for models
RUN mkdir -p /root/.local/share/tts

# Simple FastAPI wrapper for XTTS v2
COPY api.py .

EXPOSE 9880

CMD ["uvicorn", "api:app", "--host", "0.0.0.0", "--port", "9880"]
