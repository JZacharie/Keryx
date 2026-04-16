FROM pytorch/pytorch:2.3.1-cuda12.1-cudnn8-runtime

ENV DEBIAN_FRONTEND=noninteractive
RUN apt-get update && apt-get install -y --no-install-recommends \
    ffmpeg git curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY requirements.txt .
RUN pip install --no-cache-dir --upgrade pip "setuptools<70" wheel
RUN pip install --no-cache-dir -r requirements.txt

# Create cache directory and ensure it's writable
RUN mkdir -p /app/.cache/whisper && chmod -R 777 /app/.cache

COPY main.py .

ENV S3_BUCKET=keryx \
    S3_ENDPOINT=https://minio-170-api.zacharie.org \
    WHISPER_MODEL=medium \
    TRANSLATOR_BACKEND=ollama \
    PYTHONUNBUFFERED=1 \
    HOME=/app \
    WHISPER_CACHE_DIR=/app/.cache/whisper

# Volume for persistence
VOLUME ["/app/.cache/whisper"]

EXPOSE 8000
CMD ["uvicorn", "main:app", "--host", "0.0.0.0", "--port", "8000"]
