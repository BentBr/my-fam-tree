# syntax=docker/dockerfile:1.7
# Pinned via rust-toolchain.toml; the image just needs to be a recent nightly.
#
# DEV/CI ONLY — NOT published to the registry. The migrator (`run_migrations`)
# is bundled into the published api image (.docker/api.Dockerfile) for operator
# use. This standalone file exists solely for the compose `migrator` service.

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
RUN cargo build --release --bin run_migrations

FROM debian:bookworm-slim
ARG OCI_SOURCE
ARG OCI_REVISION
ARG OCI_CREATED
LABEL org.opencontainers.image.title="my-family-migrator" \
      org.opencontainers.image.description="my-family DB migrator (SQLx)" \
      org.opencontainers.image.source="${OCI_SOURCE}" \
      org.opencontainers.image.revision="${OCI_REVISION}" \
      org.opencontainers.image.created="${OCI_CREATED}"
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /app/target/release/run_migrations /usr/local/bin/run_migrations
ENTRYPOINT ["/usr/local/bin/run_migrations"]
