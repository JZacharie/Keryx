FROM pytorch/pytorch:2.3.1-cuda12.1-cudnn8-runtime

COPY --from=mwader/static-ffmpeg:7.0 /ffmpeg /usr/local/bin/
COPY --from=mwader/static-ffmpeg:7.0 /ffprobe /usr/local/bin/

ENV DEBIAN_FRONTEND=noninteractive
RUN sed -i 's/archive.ubuntu.com/mirrors.kernel.org/g' /etc/apt/sources.list && \
    sed -i 's/security.ubuntu.com/mirrors.kernel.org/g' /etc/apt/sources.list && \
    apt-get update && apt-get install -y --no-install-recommends \
    libgl1 libglib2.0-0 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY requirements.txt .
RUN pip install --no-cache-dir --upgrade pip && \
    pip install --no-cache-dir "setuptools<70"
RUN --mount=type=cache,target=/root/.cache/pip \
    pip install -r requirements.txt

COPY main.py .

ENV S3_BUCKET=keryx \
    S3_ENDPOINT=https://minio-170-api.zacharie.org \
    PYTHONUNBUFFERED=1

EXPOSE 8000
CMD ["python", "main.py"]
