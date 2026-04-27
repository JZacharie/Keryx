FROM pytorch/pytorch:2.3.1-cuda12.1-cudnn8-runtime

ENV DEBIAN_FRONTEND=noninteractive
# 1. Fix Ubuntu mirrors and install system dependencies
RUN sed -i 's/archive.ubuntu.com/mirrors.kernel.org/g' /etc/apt/sources.list && \
    sed -i 's/security.ubuntu.com/mirrors.kernel.org/g' /etc/apt/sources.list && \
    apt-get update && apt-get install -y --no-install-recommends \
    ffmpeg libgl1 libglib2.0-0 curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# 2. Install uv for fast dependency management
ENV UV_SYSTEM_PYTHON=1
RUN curl -LsSf https://astral.sh/uv/install.sh | sh && \
    mv /root/.local/bin/uv /root/.local/bin/uvx /bin/

# 3. Install dependencies
COPY requirements.txt .
RUN --mount=type=cache,target=/root/.cache/uv \
    uv pip install -r requirements.txt

COPY main.py .

ENV S3_BUCKET=keryx \
    S3_ENDPOINT=https://minio-170-api.zacharie.org \
    USE_SDXL=false \
    PYTHONUNBUFFERED=1

EXPOSE 8000
CMD ["uvicorn", "main:app", "--host", "0.0.0.0", "--port", "8000"]

