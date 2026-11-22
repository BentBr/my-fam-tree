#!/usr/bin/env bash
# Files a /qa run covers, one per line as "<class>\t<path>", after a header.
#
#   scope.sh          whole branch: merge-base with origin/main .. HEAD
#   scope.sh delta    since last push: @{u}..HEAD (whole branch if no upstream)
#
# Both modes add uncommitted (staged + unstaged) and untracked files, so a
# pushed branch with local edits is never reported as clean.
#
# Classes: rust-local:<crate> · rust-shared · fe-local · fe-shared · ci · docs
# Env (tests): QA_BASE_REF (default origin/main), QA_NO_FETCH=1 skips the fetch.
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"

mode="${1:-branch}"
base_ref="${QA_BASE_REF:-origin/main}"
if [ "${QA_NO_FETCH:-0}" != "1" ] && [ "$base_ref" = "origin/main" ]; then
    git fetch -q origin main
fi

branch_base() { git merge-base "$base_ref" HEAD; }

case "$mode" in
    branch)
        base="$(branch_base)"
        echo "# mode: branch base=$(git rev-parse --short "$base")"
        ;;
    delta)
        if upstream="$(git rev-parse --abbrev-ref --symbolic-full-name '@{u}' 2>/dev/null)"; then
            base="$(git rev-parse "$upstream")"
            echo "# mode: delta upstream=$upstream"
        else
            base="$(branch_base)"
            echo "# mode: delta (no upstream; whole branch) base=$(git rev-parse --short "$base")"
        fi
        ;;
    *)
        echo "usage: $0 [delta]" >&2
        exit 64
        ;;
esac

files="$(
    {
        git diff --name-only "$base" HEAD
        git diff --name-only HEAD
        git ls-files --others --exclude-standard
    } | sort -u
)"

if [ -z "$files" ]; then
    if [ "$mode" = "delta" ]; then
        echo "# nothing new since upstream"
    else
        echo "# no changes against $base_ref"
    fi
    exit 0
fi

classify() {
    case "$1" in
        Cargo.toml | Cargo.lock | clippy.toml | rust-toolchain.toml | deny.toml | migrations/* | .sqlx/*)
            echo "rust-shared" ;;
        crates/*)
            local rest="${1#crates/}"
            echo "rust-local:${rest%%/*}" ;;
        fe/package.json | fe/pnpm-lock.yaml | fe/eslint.config.js | fe/eslint-rules/* | fe/tsconfig* | fe/vite.config.ts | fe/vitest.config.ts)
            echo "fe-shared" ;;
        fe/*)
            echo "fe-local" ;;
        .github/* | .githooks/* | .rusty_dev_tool/* | scripts/* | .docker/* | compose.yaml)
            echo "ci" ;;
        *)
            echo "docs" ;;
    esac
}

while IFS= read -r f; do
    printf '%s\t%s\n' "$(classify "$f")" "$f"
done <<< "$files"
