# syntax=docker/dockerfile:1.7
# Pinned via rust-toolchain.toml; the image just needs to be a recent nightly.
FROM rustlang/rust:nightly-slim AS builder
WORKDIR /app
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config libssl-dev ca-certificates && rm -rf /var/lib/apt/lists/*
COPY . .
ENV SQLX_OFFLINE=true
RUN cargo build --release --bin reminder-worker

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /app/target/release/reminder-worker /usr/local/bin/reminder-worker
ENTRYPOINT ["/usr/local/bin/reminder-worker"]
