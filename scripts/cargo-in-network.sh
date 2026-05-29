#!/usr/bin/env bash
# Run a cargo command inside the compose `my-fam-tree` network so integration
# tests can reach mailpit/postgres/redis at their service-name hostnames.
#
# Usage:
#   ./scripts/cargo-in-network.sh test -p my-fam-tree-email --test mailpit
#
# The container reuses persistent docker volumes for the cargo registry, git
# checkouts, and the workspace target/ so successive runs reuse compiled deps.
set -euo pipefail

cd "$(dirname "$0")/.."

NETWORK=my-fam-tree_my-fam-tree
IMAGE=${RUST_IMAGE:-rustlang/rust:nightly}

# Bring the dependency services up so the network exists. Safe if already up.
docker compose up -d postgres redis mailpit >/dev/null

exec docker run --rm \
    --network "$NETWORK" \
    -v "$(pwd):/workspace" \
    -v my-fam-tree-target:/workspace/target \
    -v my-fam-tree-cargo-registry:/usr/local/cargo/registry \
    -v my-fam-tree-cargo-git:/usr/local/cargo/git \
    -w /workspace \
    -e DATABASE_URL=postgres://my_fam_tree:my_fam_tree@postgres:5432/my_fam_tree \
    -e REDIS_URL=redis://redis:6379/0 \
    -e EMAIL_DSN=smtp://mailpit:1025 \
    -e MAILPIT_API=http://mailpit:8025 \
    -e RUSTFLAGS="" \
    "$IMAGE" \
    cargo "$@"
