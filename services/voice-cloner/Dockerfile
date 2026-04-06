# Use PyTorch with CUDA support
FROM pytorch/pytorch:2.2.1-cuda12.1-cudnn8-runtime

WORKDIR /app

# Install system dependencies
RUN apt-get update && apt-get install -y \
    git \
    ffmpeg \
    libsndfile1 \
    && rm -rf /var/lib/apt/lists/*

# Install Coqui TTS and other dependencies
RUN pip install --no-cache-dir \
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
