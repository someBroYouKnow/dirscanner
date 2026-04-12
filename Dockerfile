# syntax=docker/dockerfile:1

ARG APP_ENV=dev

# -------------------------
# 🏗️ Base (shared setup)
# -------------------------
FROM rust:1-bookworm AS base

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

WORKDIR /app

# Cache dependencies first
COPY Cargo.toml Cargo.lock ./

# Pre-build dependencies (huge speed boost)
RUN mkdir src && echo "fn main(){}" > src/main.rs \
    && cargo build \
    && rm -rf src

# -------------------------
# 🧪 Dev Stage
# -------------------------
FROM base AS dev

# Install hot-reload tool
RUN cargo install cargo-watch

# Copy full source
COPY . .

CMD ["cargo", "watch", "-x", "run"]

# -------------------------
# 🏗️ Builder (Prod)
# -------------------------
FROM base AS builder

ARG APP_ENV

COPY src ./src

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    cargo build --release

# -------------------------
# 🚀 Runtime (Prod)
# -------------------------
FROM debian:bookworm-slim AS runtime

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        ca-certificates \
        libgtk-3-0 \
        libglib2.0-0 \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/dir-scanner /usr/local/bin/dir-scanner

CMD ["/usr/local/bin/dir-scanner"]