ARG RUST_VERSION=1.95
ARG NODE_VERSION=22

# Build the React dashboard (frontend/ -> static/app)
FROM node:${NODE_VERSION}-slim AS frontend-builder
RUN corepack enable
WORKDIR /app/frontend
COPY frontend/package.json frontend/pnpm-lock.yaml ./
RUN --mount=type=cache,target=/root/.local/share/pnpm/store \
    pnpm install --frozen-lockfile
COPY frontend/ ./
RUN pnpm build

# Base image with Rust toolchain and build deps
FROM rust:${RUST_VERSION} AS chef-base
ENV DEBIAN_FRONTEND=noninteractive
RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        build-essential pkg-config cmake \
    && rm -rf /var/lib/apt/lists/*
RUN cargo install cargo-chef
WORKDIR /app

# Plan dependency builds
FROM chef-base AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# Cache dependency compilation
FROM chef-base AS cacher
WORKDIR /app
COPY --from=planner /app/recipe.json ./recipe.json
# Use buildkit caches for registry and git to speed up builds
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    cargo chef cook --release --recipe-path recipe.json

# Build the application
FROM chef-base AS builder
WORKDIR /app
COPY . .
# Optionally seed target from cacher for a bit more speed
COPY --from=cacher /app/target /app/target
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    cargo build --release --bin lyre

# Minimal runtime image
FROM debian:trixie-slim AS runtime

ENV DEBIAN_FRONTEND=noninteractive
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates tzdata tini ffmpeg yt-dlp libopus0 aria2 curl \
    && rm -rf /var/lib/apt/lists/*

# Non-root user
ARG USER=lyre
ARG UID=10001
RUN useradd -m -u ${UID} -s /bin/bash ${USER}

# Data/cache directories (mounted as volume by default)
ENV HOME=/home/${USER}
ENV XDG_CACHE_HOME=/data/cache
ENV DOWNLOAD_FOLDER=/data/downloads

# Create data dirs with proper ownership
RUN mkdir -p /data/cache /data/downloads /data/cookies /app \
    && chown -R ${USER}:${USER} /data /app

# Drop privileges
USER ${USER}
WORKDIR /app
VOLUME ["/data"]

# Copy the compiled binary
COPY --from=builder /app/target/release/lyre /usr/local/bin/lyre

COPY ./static ./static
COPY --from=frontend-builder /app/static/app ./static/app

# Sensible defaults
ENV RUST_LOG=info
# DATABASE_URL (postgres://...) must be provided at runtime - no default, since a
# stale/wrong value fails silently instead of loudly at startup.

# Web server listens on 3000 (dashboard + REST API); Discord connection is outbound only
EXPOSE 3000
HEALTHCHECK --interval=30s --timeout=5s --start-period=20s --retries=3 \
    CMD curl -f http://127.0.0.1:3000/k8s/livez || exit 1
ENTRYPOINT ["/usr/bin/tini", "--", "/usr/local/bin/lyre"]
