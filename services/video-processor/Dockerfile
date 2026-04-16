FROM pytorch/pytorch:2.3.1-cuda12.1-cudnn8-runtime
ENV DEBIAN_FRONTEND=noninteractive


# Prevent Python from writing .pyc files & buffer stdout/stderr
ENV PYTHONDONTWRITEBYTECODE=1
ENV PYTHONUNBUFFERED=1

# HuggingFace & Whisper cache directory (mapped to a volume in docker-compose)
ENV HF_HOME=/app/huggingface
ENV WHISPER_CACHE=/app/whisper_models

WORKDIR /app

# 1. Install system dependencies (rarely change)
RUN apt-get update && apt-get install -y --no-install-recommends \
    libgl1 \
    libglib2.0-0 \
    ffmpeg \
    git \
    && rm -rf /var/lib/apt/lists/*

# 2. Install uv for fast dependency management
ENV UV_SYSTEM_PYTHON=1
COPY --from=ghcr.io/astral-sh/uv:latest /uv /uvx /bin/

# 3. Install dependencies
RUN uv pip install torch transformers openai-whisper opencv-python-headless
COPY requirements.txt .
RUN uv pip install -r requirements.txt

# 4. Final step: Copy only the application code
COPY main.py .

# Add non-root user
RUN useradd -m keryx && chown -R keryx:keryx /app
USER keryx

# Add command to run the processor
CMD ["python", "main.py"]
