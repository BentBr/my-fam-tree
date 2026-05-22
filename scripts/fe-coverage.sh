#!/usr/bin/env bash
# Frontend coverage wrapper. Single source of truth for the FE
# coverage invocation and the minimum threshold CI enforces. Used by
# `.github/workflows/ci.yml` and (via `rdt coverage`) by local dev.
#
# Usage:
#   scripts/fe-coverage.sh run        # run vitest with --coverage
#   scripts/fe-coverage.sh threshold  # print the threshold integer

set -euo pipefail
cd "$(dirname "$0")/.."

MIN_LINES=80

cmd="${1:-help}"

case "$cmd" in
    run)
        ./scripts/fe-in-container.sh coverage
        ;;
    threshold)
        echo "$MIN_LINES"
        ;;
    *)
        echo "usage: $0 {run|threshold}" >&2
        exit 1
        ;;
esac
