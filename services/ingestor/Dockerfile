# --- Runner ---
FROM debian:trixie-slim AS runner

# Install yt-dlp, ffmpeg and standard CA certs
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
    curl \
    ffmpeg \
    python3 \
    python3-pip \
    nodejs \
    aria2 \
    && curl -L https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp -o /usr/local/bin/yt-dlp \
    && chmod a+rx /usr/local/bin/yt-dlp \
    && rm -rf /var/lib/apt/lists/* \
    && update-ca-certificates

WORKDIR /app
RUN useradd -r -s /bin/false keryx && chown -R keryx:keryx /app

ENV REDIS_URL=redis://localhost:6379
ENV S3_BUCKET=keryx-raw
ENV S3_REGION=eu-west-1

EXPOSE 3000

# --- CI Build (uses pre-built binary) ---
FROM runner AS release-ci
ARG PREBUILT_BINARY
COPY ${PREBUILT_BINARY} /usr/local/bin/keryx-ingestor
COPY static static
RUN chmod +x /usr/local/bin/keryx-ingestor
USER keryx
CMD ["/usr/local/bin/keryx-ingestor"]

# --- Full Build ---
FROM lukemathwalker/cargo-chef:latest-rust-1.80-slim AS chef
WORKDIR /app
RUN apt-get update && apt-get install -y pkg-config libssl-dev curl && rm -rf /var/lib/apt/lists/*

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .
RUN cargo build --release --bin keryx-ingestor

FROM runner AS release
COPY --from=builder /app/target/release/keryx-ingestor /usr/local/bin/keryx-ingestor
COPY --from=builder /app/static static
RUN chown -R keryx:keryx /app
USER keryx
CMD ["/usr/local/bin/keryx-ingestor"]
