FROM python:3.11-slim

ENV DEBIAN_FRONTEND=noninteractive
RUN apt-get update && apt-get install -y --no-install-recommends \
    ffmpeg git curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY requirements.txt .
RUN pip install --no-cache-dir --upgrade pip "setuptools<70" wheel
RUN pip install --no-cache-dir --no-build-isolation -r requirements.txt

COPY main.py .

ENV S3_BUCKET=keryx \
    S3_ENDPOINT=https://minio-170-api.zacharie.org \
    WHISPER_MODEL=medium \
    TRANSLATOR_BACKEND=ollama \
    PYTHONUNBUFFERED=1

# Whisper télécharge les modèles au démarrage, volume recommandé
VOLUME ["/root/.cache/whisper"]

EXPOSE 8012
CMD ["uvicorn", "main:app", "--host", "0.0.0.0", "--port", "8012"]
