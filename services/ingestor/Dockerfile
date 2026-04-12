# --- Runner ---
FROM ubuntu:24.04 AS runner

# Set non-interactive for apt
ENV DEBIAN_FRONTEND=noninteractive

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
    libopencv-dev \
    && curl -L https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp -o /usr/local/bin/yt-dlp \
    && chmod a+rx /usr/local/bin/yt-dlp \
    && update-ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Ensure user keryx exists with UID 1000, or reuse existing user with that UID
RUN if ! getent group 1000; then groupadd -g 1000 keryx; fi && \
    if ! getent passwd 1000; then useradd -r -u 1000 -g 1000 -s /bin/false keryx; fi && \
    chown -R 1000:1000 /app

ENV REDIS_URL=redis://localhost:6379 \
    S3_BUCKET=keryx-raw \
    S3_REGION=eu-west-1 \
    RUST_LOG=info

EXPOSE 3000

# --- CI Build (uses pre-built binary) ---
FROM runner AS release-ci
ARG PREBUILT_BINARY
COPY --chmod=755 ${PREBUILT_BINARY} /usr/local/bin/keryx-ingestor
COPY --chown=keryx:keryx static static
COPY --chown=keryx:keryx begin.mp4 begin.mp4
USER keryx
CMD ["/usr/local/bin/keryx-ingestor"]

# --- Full Build ---
FROM lukemathwalker/cargo-chef:latest-rust-1.91.1-slim AS chef
WORKDIR /app
RUN apt-get update && apt-get install -y pkg-config libssl-dev curl clang libclang-dev llvm-dev libopencv-dev mold sccache \
    && rm -rf /var/lib/apt/lists/*

ENV RUSTC_WRAPPER="sccache"
ENV SCCACHE_DIR="/app/.sccache"


FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN --mount=type=cache,target=/app/.sccache \
    cargo chef cook --release --recipe-path recipe.json
COPY . .
RUN --mount=type=cache,target=/app/.sccache \
    cargo build --release --bin keryx-ingestor

FROM runner AS release
COPY --from=builder --chmod=755 /app/target/release/keryx-ingestor /usr/local/bin/keryx-ingestor
COPY --from=builder --chown=keryx:keryx /app/services/ingestor/static static
COPY --from=builder --chown=keryx:keryx /app/services/ingestor/begin.mp4 begin.mp4
USER keryx
CMD ["/usr/local/bin/keryx-ingestor"]

