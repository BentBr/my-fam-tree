#!/usr/bin/env bash
# Single owner of "generate the OpenAPI spec" and "check it for drift".
#
#   ./scripts/openapi.sh check   # exit non-zero on drift OR generator failure
#   ./scripts/openapi.sh write   # replace the spec only if generation succeeds
#
# The generator writes to a temp file first, so a failing build never
# truncates the committed spec and never hides behind a pipe's exit status.
#
# Overrides (used by crates/arch-tests/tests/openapi_script.rs):
#   OPENAPI_SPEC      spec path            (default fe/openapi.json)
#   OPENAPI_DUMP_CMD  generator command    (default cargo run … openapi-dump)
set -euo pipefail

cd "$(dirname "$0")/.."

mode="${1:-}"
case "$mode" in
    check | write) ;;
    *)
        echo "usage: $0 {check|write}" >&2
        exit 64
        ;;
esac

export OPENAPI_SPEC="${OPENAPI_SPEC:-fe/openapi.json}"
dump_cmd="${OPENAPI_DUMP_CMD:-cargo run -q -p my-fam-tree-openapi --bin openapi-dump}"

tmp="$(mktemp)"
trap 'rm -f "$tmp"' EXIT

status=0
bash -c "$dump_cmd" > "$tmp" || status=$?
if [ "$status" -ne 0 ]; then
    echo "[openapi] generator failed (exit $status); $OPENAPI_SPEC left untouched" >&2
    exit "$status"
fi

if [ "$mode" = "check" ]; then
    if ! diff -u "$OPENAPI_SPEC" "$tmp"; then
        echo "[openapi] $OPENAPI_SPEC is stale; run: rdt openapi" >&2
        exit 1
    fi
    echo "[openapi] no drift"
else
    # Copy contents (not mv) so the spec keeps its mode; mktemp files are 0600.
    cat "$tmp" > "$OPENAPI_SPEC"
    echo "[openapi] wrote $OPENAPI_SPEC"
fi
