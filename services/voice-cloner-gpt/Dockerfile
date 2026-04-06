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
    pip install "transformers>=4.45.0,<5.0" "peft>=0.12.0" "huggingface_hub<0.24" && \
    pip install -r requirements.txt \
    fastapi \
    uvicorn \
    requests \
    dm-tree>=0.1.8 \
    x-transformers \
    f5-tts

# 4. Download pretrained models
RUN pip install "huggingface_hub>=0.24" && \
    huggingface-cli download lj1995/GPT-SoVITS \
        chinese-roberta-wwm-ext-large/config.json \
        chinese-roberta-wwm-ext-large/tokenizer.json \
        chinese-roberta-wwm-ext-large/pytorch_model.bin \
        chinese-hubert-base/config.json \
        chinese-hubert-base/preprocessor_config.json \
        chinese-hubert-base/pytorch_model.bin \
        gsv-v2final-pretrained/s2G2333k.pth \
        gsv-v2final-pretrained/s2D2333k.pth \
        gsv-v2final-pretrained/s1bert25hz-5kh-longer-epoch=12-step=369668.ckpt \
        --local-dir /app/GPT_SoVITS/pretrained_models && \
    pip install "huggingface_hub<0.24"

# 5. Final step: Copy my API wrapper
COPY gpt_api.py .

# Models will be mounted as volumes
# /app/GPT_SoVITS/pretrained_models
# /app/GPT_weights
# /app/SoVITS_weights

EXPOSE 9880

# Run my API
CMD ["python", "gpt_api.py"]
