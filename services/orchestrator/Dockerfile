# --- Runner ---
FROM ubuntu:24.04 AS runner

ENV DEBIAN_FRONTEND=noninteractive

# Minimal dependencies for the orchestrator (no more heavy ML/CV local)
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
    curl \
    && update-ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Ensure user keryx exists with UID 1000
RUN if getent passwd 1000; then \
        existing_user=$(getent passwd 1000 | cut -d: -f1); \
        usermod -l keryx $existing_user; \
    else \
        groupadd -g 1000 keryx && \
        useradd -r -u 1000 -g keryx -s /bin/false keryx; \
    fi && \
    chown -R 1000:1000 /app

ENV RUST_LOG=info
EXPOSE 3000

# --- Full Build ---
FROM ubuntu:24.04 AS chef
WORKDIR /app

# Install system dependencies and Rust
RUN apt-get update && apt-get install -y \
    pkg-config libssl-dev curl clang libclang-dev llvm-dev mold sccache ca-certificates \
    && curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable \
    && rm -rf /var/lib/apt/lists/*

ENV PATH="/root/.cargo/bin:${PATH}"
RUN cargo install cargo-chef --version 0.1.77

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
COPY --from=builder --chown=keryx:keryx /app/services/orchestrator/static static
COPY --from=builder --chown=keryx:keryx /app/services/orchestrator/begin.mp4 begin.mp4
USER keryx
CMD ["/usr/local/bin/keryx-ingestor"]
FROM runner AS release-ci
ARG PREBUILT_BINARY
COPY static static
COPY begin.mp4 begin.mp4
COPY ${PREBUILT_BINARY} /usr/local/bin/keryx-ingestor
RUN chmod 755 /usr/local/bin/keryx-ingestor
USER keryx
CMD ["/usr/local/bin/keryx-ingestor"]
