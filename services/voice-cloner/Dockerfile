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

# Install heavy TTS dependencies first (rarely change)
RUN --mount=type=cache,target=/root/.cache/pip \
    pip install "transformers<4.40" "TTS>=0.22.0"

# Install remaining dependencies
COPY requirements.txt .
RUN --mount=type=cache,target=/root/.cache/pip \
    pip install -r requirements.txt

# Add non-root user
RUN useradd -m keryx && chown -R keryx:keryx /app
USER keryx

EXPOSE 9880
CMD ["uvicorn", "api:app", "--host", "0.0.0.0", "--port", "9880"]
