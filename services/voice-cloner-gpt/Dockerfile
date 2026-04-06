# syntax=docker/dockerfile:1
FROM pytorch/pytorch:2.2.1-cuda12.1-cudnn8-runtime
ENV DEBIAN_FRONTEND=noninteractive


WORKDIR /app

# 1. System dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    ffmpeg \
    git \
    libsndfile1 \
    cmake \
    build-essential \
    && rm -rf /var/lib/apt/lists/*

# 2. GPT-SoVITS setup
RUN git clone https://github.com/RVC-Boss/GPT-SoVITS.git .

# 3. Install requirements with cache
RUN --mount=type=cache,target=/root/.cache/pip \
    pip install -r requirements.txt \
    fastapi \
    uvicorn \
    requests

# 4. Final step: Copy my API wrapper
COPY gpt_api.py .

# Models will be mounted as volumes
# /app/GPT_SoVITS/pretrained_models
# /app/GPT_weights
# /app/SoVITS_weights

EXPOSE 9880

# Run my API
CMD ["python", "gpt_api.py"]
