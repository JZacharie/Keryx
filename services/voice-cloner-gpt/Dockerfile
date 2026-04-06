# syntax=docker/dockerfile:1
FROM pytorch/pytorch:2.2.1-cuda12.1-cudnn8-runtime
ENV DEBIAN_FRONTEND=noninteractive


WORKDIR /app

# 1. System dependencies
RUN --mount=type=cache,target=/var/cache/apt,sharing=locked \
    --mount=type=cache,target=/var/lib/apt/lists,sharing=locked \
    apt-get update && apt-get install -y --no-install-recommends \
    ffmpeg \
    git \
    libsndfile1 \
    cmake \
    build-essential

# 2. GPT-SoVITS setup
# Note: Cloning here is convenient but less deterministic for caching.
# We keep it as per the current setup but ensure it's in its own layer.
RUN git clone https://github.com/RVC-Boss/GPT-SoVITS.git .

# 3. Install requirements with cache
RUN --mount=type=cache,target=/root/.cache/pip \
    sed -i '/transformers/d; /peft/d' requirements.txt && \
    pip uninstall -y torchaudio && \
    pip install torchaudio==2.2.1 --index-url https://download.pytorch.org/whl/cu121 && \
    pip install "transformers>=4.45.0" "peft>=0.12.0" && \
    pip install -r requirements.txt \
    fastapi \
    uvicorn \
    requests \
    dm-tree>=0.1.8 \
    x-transformers \
    f5-tts

# 4. Final step: Copy my API wrapper
COPY gpt_api.py .

# Models will be mounted as volumes
# /app/GPT_SoVITS/pretrained_models
# /app/GPT_weights
# /app/SoVITS_weights

EXPOSE 9880

# Run my API
CMD ["python", "gpt_api.py"]
