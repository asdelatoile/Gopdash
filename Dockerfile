# syntax=docker/dockerfile:1

# ── Stage 1: Build frontend ──────────────────────────────────────────────────
FROM node:22-alpine AS frontend-builder

WORKDIR /app/frontend
COPY frontend/package.json frontend/package-lock.json* ./
RUN npm ci 2>/dev/null || npm install

COPY frontend/ ./
RUN npm run build

# ── Stage 2: Build backend ───────────────────────────────────────────────────
FROM rust:1.88-slim-bookworm AS backend-builder

RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY backend/Cargo.toml backend/Cargo.lock* ./
COPY backend/src ./src

RUN cargo build --release

# ── Stage 3: Runtime (~50 MB) ────────────────────────────────────────────────
FROM debian:bookworm-slim AS runtime

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates wget \
    && rm -rf /var/lib/apt/lists/* \
    && useradd -r -s /bin/false gopdash

WORKDIR /app

COPY --from=backend-builder /app/target/release/gopdash /usr/local/bin/gopdash
COPY --from=frontend-builder /app/frontend/build /app/static

RUN mkdir -p /data && chown gopdash:gopdash /data /app

ENV CONFIG_DIR=/data/config \
    STATIC_DIR=/app/static \
    HOST=0.0.0.0 \
    PORT=8080 \
    RUST_LOG=gopdash=info

EXPOSE 8080

VOLUME ["/data"]

HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD wget -qO- http://127.0.0.1:8080/api/health || exit 1

# Run as root: required for read-only Docker socket access (root:root on Docker Desktop)
ENTRYPOINT ["/usr/local/bin/gopdash"]
