FROM python:3.11-slim
WORKDIR /app
# 1. Install dependencies first
COPY requirements.txt .
RUN --mount=type=cache,target=/root/.cache/pip \
    pip install -r requirements.txt

# 2. Copy application code
COPY main.py .
# Add non-root user
RUN useradd -m keryx && chown -R keryx:keryx /app
USER keryx

EXPOSE 8000
CMD ["uvicorn", "main:app", "--host", "0.0.0.0", "--port", "8000"]
