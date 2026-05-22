# syntax=docker/dockerfile:1.7
# Pinned via rust-toolchain.toml; the image just needs to be a recent nightly.

# OCI image metadata (overridden by CI via --build-arg).
ARG OCI_SOURCE="https://github.com/BentBr/my-family"
ARG OCI_REVISION=""
ARG OCI_CREATED=""

FROM rustlang/rust:nightly-slim AS builder
WORKDIR /app
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config libssl-dev ca-certificates curl && rm -rf /var/lib/apt/lists/*
COPY . .
ENV SQLX_OFFLINE=true
# Build both the long-running api server and the one-shot seed binary in the
# same image — they share the same workspace so this is a single compile pass.
RUN cargo build --release --bin api --bin seed

FROM debian:bookworm-slim
ARG OCI_SOURCE
ARG OCI_REVISION
ARG OCI_CREATED
LABEL org.opencontainers.image.title="my-family-api" \
      org.opencontainers.image.description="my-family HTTP API (Actix-web)" \
      org.opencontainers.image.source="${OCI_SOURCE}" \
      org.opencontainers.image.revision="${OCI_REVISION}" \
      org.opencontainers.image.created="${OCI_CREATED}"
# `wget` is required by the compose healthcheck (`wget -qO- /api/v1/health`).
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates wget && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /app/target/release/api /usr/local/bin/api
COPY --from=builder /app/target/release/seed /usr/local/bin/seed
EXPOSE 8080
ENTRYPOINT ["/usr/local/bin/api"]
