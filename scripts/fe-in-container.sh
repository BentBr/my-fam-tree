#!/usr/bin/env bash
# Run any pnpm command inside the `fe` compose service.
# Usage: scripts/fe-in-container.sh install
#        scripts/fe-in-container.sh lint
#        scripts/fe-in-container.sh openapi-codegen
set -euo pipefail
cd "$(dirname "$0")/.."
# `docker compose` interpolates env vars across the WHOLE file before launching
# anything, so we provide harmless placeholders for the api service's required
# JWT_* vars. `--no-deps` ensures the api service is never actually started; the
# placeholders only satisfy interpolation. Real keys in `.env` always take
# precedence (dotenvy/compose honor the first definition).
export JWT_PRIVATE_KEY="${JWT_PRIVATE_KEY:-fe-wrapper-placeholder}"
export JWT_PRIVATE_KEY_ID="${JWT_PRIVATE_KEY_ID:-fe-wrapper-placeholder}"
export JWT_PUBLIC_KEYS="${JWT_PUBLIC_KEYS:-fe-wrapper-placeholder}"
exec docker compose run --rm --no-deps -T fe sh -c "corepack enable && corepack prepare pnpm@9.12.0 --activate && pnpm $*"
