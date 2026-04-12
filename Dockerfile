# syntax=docker/dockerfile:1
# BuildKit features (cache mounts) need Docker BuildKit enabled (default in recent Docker Desktop).

# Declared before the first FROM so you can override at build time: docker build --build-arg APP_ENV=production .
# Default keeps local/dev behaviour (debug build, faster compiles).
ARG APP_ENV=dev

# --- Stage 1: build ---
# Official Rust image on Debian Bookworm; includes cargo, rustc, and a C toolchain for native deps (e.g. rfd).
FROM rust:1-bookworm AS builder

# Native deps for crates like wayland-sys / GTK (pulled in by rfd on Linux). Runtime stage only needs shared libs; build needs -dev + pkg-config.
RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        pkg-config \
        libwayland-dev \
        libgtk-3-dev \
        libglib2.0-dev \
        libatk1.0-dev \
        libcairo2-dev \
        libgdk-pixbuf-2.0-dev \
        libpango1.0-dev \
    && rm -rf /var/lib/apt/lists/*

# Re-declare ARG after FROM so this stage receives the value passed from docker build / compose build.args.
ARG APP_ENV

# Working directory inside the container for all following COPY and RUN steps.
WORKDIR /app

# Copy dependency manifests first so Docker can cache this layer when only source changes.
COPY Cargo.toml Cargo.lock ./

# Application source.
COPY src ./src

# Cache Cargo registry/git and target dir between builds (speeds repeat builds). Build locked to Cargo.lock for reproducibility.
# If APP_ENV is production, build optimized release binary; otherwise debug. Copy artifact to a fixed path for the next stage.
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/app/target \
    if [ "$APP_ENV" = "production" ]; then \
      cargo build --locked --release && cp target/release/dir-scanner /app/dir-scanner; \
    else \
      cargo build --locked && cp target/debug/dir-scanner /app/dir-scanner; \
    fi

# --- Stage 2: runtime ---
# Small Debian image with only what is needed to run the binary (no Rust toolchain).
FROM debian:bookworm-slim AS runtime

# Same ARG/ENV so the running container can read APP_ENV (e.g. if you later branch on it in Rust via std::env::var).
ARG APP_ENV
ENV APP_ENV=${APP_ENV}

# rfd / GTK stack needs shared libraries at runtime on Linux.
RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        ca-certificates \
        libgtk-3-0 \
        libglib2.0-0 \
    && rm -rf /var/lib/apt/lists/*

# Install the compiled binary from the builder stage.
COPY --from=builder /app/dir-scanner /usr/local/bin/dir-scanner

# Default command runs the scanner. Override with docker compose run ... <cmd> if needed.
CMD ["/usr/local/bin/dir-scanner"]
