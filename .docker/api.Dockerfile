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
# Build the api plus the operator tooling (migrator + seeder) in one cargo
# invocation so they share the dependency/codegen cache. All three binaries
# are bundled into the runtime image; only `api` is the default command.
RUN cargo build --release --bin api --bin run_migrations --bin seed

FROM debian:bookworm-slim
ARG OCI_SOURCE
ARG OCI_REVISION
ARG OCI_CREATED
LABEL org.opencontainers.image.title="my-family-api" \
      org.opencontainers.image.description="my-family HTTP API (Actix-web); bundles run_migrations + seed for operators" \
      org.opencontainers.image.source="${OCI_SOURCE}" \
      org.opencontainers.image.revision="${OCI_REVISION}" \
      org.opencontainers.image.created="${OCI_CREATED}"
# `wget` is required by the compose healthcheck (`wget -qO- /api/v1/health`).
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates wget && rm -rf /var/lib/apt/lists/*
# Land interactive shells (`docker compose exec api bash`) where the bundled
# binaries actually live — the slim runtime has no /app source tree.
WORKDIR /usr/local/bin
# The api server is the default command. The migrator (`run_migrations`) and
# seeder (`seed`) binaries ride along so an operator can run them one-shot
# against a deployed stack, e.g.:
#   docker run --rm -e DATABASE_URL=... --entrypoint run_migrations <api-image>
#   docker run --rm --env-file prod.env  --entrypoint seed          <api-image>
# `run_migrations` reads DATABASE_URL from the env (or a --database-url flag);
# `seed` loads the full app Config from the env (DATABASE_URL + JWT_* etc.),
# matching the compose `seeder` service. The seeder is intended for dev/CI.
COPY --from=builder /app/target/release/api /usr/local/bin/api
COPY --from=builder /app/target/release/run_migrations /usr/local/bin/run_migrations
COPY --from=builder /app/target/release/seed /usr/local/bin/seed
EXPOSE 8080
ENTRYPOINT ["/usr/local/bin/api"]
