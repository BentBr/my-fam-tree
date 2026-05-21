#!/usr/bin/env bash
# Run any pnpm command inside the `fe` compose service.
# Usage: scripts/fe-in-container.sh install
#        scripts/fe-in-container.sh lint
#        scripts/fe-in-container.sh openapi-codegen
#        scripts/fe-in-container.sh test:e2e   # routes to the playwright service
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

# Playwright browsers are too large for the `fe` (node:alpine) image, and the
# alpine musl runtime can't execute them anyway. The dedicated `playwright`
# service ships the Microsoft-pinned chromium build. Route any e2e command —
# `test:e2e`, `test:e2e:headed`, `exec playwright …`, etc. — there instead.
# The service stays up between calls (`tail -f /dev/null`), so we `exec` into
# the running container rather than spawning a new one — much faster.
case "${1:-}" in
    test:e2e | test:e2e:* | playwright | exec)
        if [[ "$(docker compose ps -q playwright 2>/dev/null)" == "" ]]; then
            docker compose up -d playwright >/dev/null
        fi
        exec docker compose exec -T playwright pnpm "$@"
        ;;
esac

exec docker compose run --rm --no-deps -T fe sh -c "corepack enable && corepack prepare pnpm@10.33.4 --activate && pnpm $*"
