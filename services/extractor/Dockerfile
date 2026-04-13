FROM python:3.11-slim

ENV DEBIAN_FRONTEND=noninteractive
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates curl ffmpeg aria2 nodejs \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY requirements.txt .
RUN pip install --no-cache-dir --upgrade pip && \
    pip install --no-cache-dir "setuptools<70"
RUN pip install --no-cache-dir -r requirements.txt

COPY main.py .

ENV S3_BUCKET=keryx \
    S3_ENDPOINT=https://minio-170-api.zacharie.org \
    PYTHONUNBUFFERED=1

EXPOSE 8010
CMD ["uvicorn", "main:app", "--host", "0.0.0.0", "--port", "8010"]
