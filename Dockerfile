ARG RUST_VERSION=1.89

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
    && apt-get install -y --no-install-recommends ca-certificates tzdata tini ffmpeg yt-dlp libopus0 \
    && rm -rf /var/lib/apt/lists/*

# Non-root user
ARG USER=lyre
ARG UID=10001
RUN useradd -m -u ${UID} -s /bin/bash ${USER}
WORKDIR /app

# Data/cache directories (mounted as volume by default)
ENV HOME=/home/${USER}
ENV XDG_CACHE_HOME=/data/cache
ENV DOWNLOAD_FOLDER=/data/downloads
# Create data dirs with proper ownership, then drop privileges
RUN mkdir -p /data/cache /data/downloads \
    && chown -R ${USER}:${USER} /data
USER ${USER}
VOLUME ["/data"]

# Copy the compiled binary
COPY --from=builder /app/target/release/lyre /usr/local/bin/lyre

# Sensible defaults
ENV RUST_LOG=info

# No ports exposed (Discord bot is outbound only)
# Web server listens on 3000
EXPOSE 3000
ENTRYPOINT ["/usr/bin/tini", "--", "/usr/local/bin/lyre"]
