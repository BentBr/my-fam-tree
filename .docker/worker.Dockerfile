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
# CARGO_FEATURES is empty for production (the published image MUST stay
# feature-free). E2E/dev builds pass `--build-arg CARGO_FEATURES=test-fixtures`
# (via compose's WORKER_FEATURES) to enable the clock-advance HTTP
# listener. The `${VAR:+--features "$VAR"}` form expands to nothing when unset.
ARG CARGO_FEATURES=""
RUN cargo build --release --bin worker ${CARGO_FEATURES:+--features "$CARGO_FEATURES"}

FROM debian:trixie-slim
ARG OCI_SOURCE
ARG OCI_REVISION
ARG OCI_CREATED
LABEL org.opencontainers.image.title="my-family-worker" \
      org.opencontainers.image.description="my-family reminder worker (background tick loop)" \
      org.opencontainers.image.source="${OCI_SOURCE}" \
      org.opencontainers.image.revision="${OCI_REVISION}" \
      org.opencontainers.image.created="${OCI_CREATED}"
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR /usr/local/bin
COPY --from=builder /app/target/release/worker /usr/local/bin/worker
# Declared so the dinghy reverse-proxy (docker-gen) builds a vhost upstream for
# the worker. Only actually listened on in `test-fixtures` builds (the
# clock-advance endpoint); a normal build leaves it idle. Harmless in prod.
EXPOSE 9091
ENTRYPOINT ["/usr/local/bin/worker"]
