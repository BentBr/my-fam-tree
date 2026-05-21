#!/usr/bin/env bash
# Workspace-wide `cargo llvm-cov` wrapper. Single source of truth for what
# we exclude from coverage measurement and the minimum threshold CI enforces.
# Used by `.github/workflows/ci.yml` and (via `rdt coverage`) by local dev.
#
# Usage:
#   scripts/coverage.sh profile    # run tests with profiling (no report)
#   scripts/coverage.sh lcov       # emit coverage/rust.lcov
#   scripts/coverage.sh json       # emit coverage/rust.json
#   scripts/coverage.sh summary    # print human-readable per-file table
#   scripts/coverage.sh check      # fail if line coverage < threshold
#   scripts/coverage.sh all        # profile + lcov + json + summary + check
#   scripts/coverage.sh threshold  # print the threshold integer
#
# Adding a new "untestable wiring" file: extend the IGNORE regex below.

set -euo pipefail

cd "$(dirname "$0")/.."

# Exclude code that's effectively integration glue / runtime wiring and
# cannot be meaningfully unit-tested:
#   - binary entry points (src/bin/*, src/main.rs)
#   - tracing subscriber init (api/src/tracing_setup.rs)
#   - cache/email/openapi wiring (pool/smtp/derive-only lib.rs)
#   - aggregated OpenAPI doc (api/src/openapi_doc.rs is derive-only; the
#     openapi-dump binary serialises it but binaries are already excluded)
IGNORE='(/bin/|/main\.rs$|/tracing_setup\.rs$|/cache/src/pool\.rs$|/email/src/smtp\.rs$|/openapi/src/lib\.rs$|/api/src/openapi_doc\.rs$)'

# Minimum line-coverage threshold (percent). CI fails the build below this.
MIN_LINES=80

cmd="${1:-help}"

case "$cmd" in
    profile)
        cargo llvm-cov --workspace --no-report --ignore-filename-regex "$IGNORE"
        ;;
    lcov)
        mkdir -p coverage
        cargo llvm-cov report --lcov --output-path coverage/rust.lcov --ignore-filename-regex "$IGNORE"
        ;;
    json)
        mkdir -p coverage
        cargo llvm-cov report --json --output-path coverage/rust.json --ignore-filename-regex "$IGNORE"
        ;;
    summary)
        cargo llvm-cov report --summary-only --ignore-filename-regex "$IGNORE"
        ;;
    check)
        cargo llvm-cov report --fail-under-lines "$MIN_LINES" --ignore-filename-regex "$IGNORE"
        ;;
    all)
        "$0" profile
        "$0" lcov
        "$0" json
        "$0" summary
        "$0" check
        ;;
    threshold)
        echo "$MIN_LINES"
        ;;
    *)
        echo "usage: $0 {profile|lcov|json|summary|check|all|threshold}" >&2
        exit 1
        ;;
esac
