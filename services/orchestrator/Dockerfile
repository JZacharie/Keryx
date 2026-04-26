# --- Runner base image ---
FROM ubuntu:24.04 AS runner

ENV DEBIAN_FRONTEND=noninteractive

RUN sed -i 's/archive.ubuntu.com/mirrors.kernel.org/g' /etc/apt/sources.list && \
    sed -i 's/security.ubuntu.com/mirrors.kernel.org/g' /etc/apt/sources.list && \
    apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
    && update-ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

RUN if getent passwd 1000; then \
    existing_user=$(getent passwd 1000 | cut -d: -f1); \
    usermod -l keryx "$existing_user"; \
    groupmod -n keryx "$existing_user" 2>/dev/null || true; \
    else \
    groupadd -g 1000 keryx && \
    useradd -r -u 1000 -g keryx -s /bin/false keryx; \
    fi && \
    chown -R 1000:1000 /app

ENV RUST_LOG=info
EXPOSE 3000

# --- Build base with Rust + cargo-chef ---
FROM ubuntu:24.04 AS chef
WORKDIR /app

RUN sed -i 's/archive.ubuntu.com/mirrors.kernel.org/g' /etc/apt/sources.list.d/ubuntu.sources || \
    sed -i 's/archive.ubuntu.com/mirrors.kernel.org/g' /etc/apt/sources.list && \
    sed -i 's/security.ubuntu.com/mirrors.kernel.org/g' /etc/apt/sources.list.d/ubuntu.sources || \
    sed -i 's/security.ubuntu.com/mirrors.kernel.org/g' /etc/apt/sources.list && \
    apt-get update && apt-get install -y \
    pkg-config libssl-dev curl clang libclang-dev mold ca-certificates \
    && curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable \
    && rm -rf /var/lib/apt/lists/*

ENV PATH="/root/.cargo/bin:${PATH}"
RUN cargo install cargo-chef --version 0.1.77 --locked

# --- Compute dependency recipe ---
FROM chef AS planner
# Only copy files needed for dependency analysis
COPY Cargo.toml Cargo.lock ./
COPY services/keryx-core services/keryx-core
COPY services/orchestrator services/orchestrator
RUN cargo chef prepare --recipe-path recipe.json

# --- Cook (cache deps), then build binary ---
FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN --mount=type=cache,target=/root/.cargo/registry \
    cargo chef cook --release --recipe-path recipe.json
# Copy actual source code
COPY Cargo.toml Cargo.lock ./
COPY services/keryx-core services/keryx-core
COPY services/orchestrator services/orchestrator
RUN --mount=type=cache,target=/root/.cargo/registry \
    cargo build --release --bin keryx-orchestrator

# --- Minimal runtime image ---
FROM runner AS release
COPY --from=builder --chmod=755 /app/target/release/keryx-orchestrator /usr/local/bin/keryx-orchestrator
COPY --from=builder --chown=1000:1000 /app/services/orchestrator/static static
COPY --from=builder --chown=1000:1000 /app/services/orchestrator/static static
# Run as root to have permission on /var/run/docker.sock in local dev
CMD ["/usr/local/bin/keryx-orchestrator"]
