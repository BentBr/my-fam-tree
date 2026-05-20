# syntax=docker/dockerfile:1.7
# Pinned via rust-toolchain.toml; the image just needs to be a recent nightly.
FROM rustlang/rust:nightly-slim AS builder
WORKDIR /app
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config libssl-dev ca-certificates curl && rm -rf /var/lib/apt/lists/*
COPY . .
ENV SQLX_OFFLINE=true
RUN cargo build --release --bin api

FROM debian:bookworm-slim
# `wget` is required by the compose healthcheck (`wget -qO- /api/v1/health`).
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates wget && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /app/target/release/api /usr/local/bin/api
EXPOSE 8080
ENTRYPOINT ["/usr/local/bin/api"]
