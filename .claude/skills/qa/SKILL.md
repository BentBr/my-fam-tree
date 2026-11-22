---
name: qa
description: Validate the current change before commit or push — scope the changed files, run the targeted gates with honest exit codes, then get a read-only review from the quality-assurance agent. Default covers the whole branch; "delta" covers only what is new since the last push.
argument-hint: "[delta]"
disable-model-invocation: true
---

# /qa — validate the current change

## 1. Scope (once)

Run `.claude/skills/qa/scripts/scope.sh` (default: whole branch from the merge base with
`origin/main`) or `.claude/skills/qa/scripts/scope.sh delta` (since `@{u}`; whole branch when
there is no upstream). Both modes include uncommitted and untracked files. Output is a header
plus one `<class>\t<path>` line per file.

- `# nothing new since upstream` → say exactly that and stop. Never report it as "clean".
- Show `git diff --stat` for the same range.

## 2. Targeted checks

Run each command on its own and take the exit code from the command itself — never from a
`| tail` / `| grep` / `&& echo OK` chain. Capture `rc=$?` straight after the command when you
need to keep going. Record every command with its exit code.

| class in scope | run |
|---|---|
| `rust-local:<crate>` | gate clippy flags on `-p my-fam-tree-<crate>`; `cargo test -p my-fam-tree-<crate>` |
| `rust-shared` (Cargo.*, clippy.toml, toolchain, deny.toml, migrations, .sqlx) | **workspace-wide** clippy gate + `cargo test --workspace` |
| any Rust, docs, ci | `cargo test -p my-fam-tree-arch-tests` (always) |
| `fe-local` | `./scripts/fe-in-container.sh lint`, `typecheck`, `format:check`, related test files |
| `fe-shared` (package.json, lockfile, eslint config/rules, tsconfig, vite/vitest config) | **full** FE `format:check`, `lint`, `typecheck`, `test` |
| routes / ApiError / response types changed | `./scripts/openapi.sh check` |
| `migrations/**` | `rdt migrate-check` |
| `ci` | `cargo machete`; `./scripts/check-file-size.sh` |

Clippy gate flags: `--all-targets --all-features -- -D clippy::all -D warnings -D clippy::pedantic
-D clippy::nursery` (with `cargo +nightly clippy`). Without a reachable DB use `SQLX_OFFLINE=true`.
Tests that need compose hostnames go through `./scripts/cargo-in-network.sh`.

## 3. Review

Hand the `quality-assurance` agent: the scope output, the diff stat, the base ref, and every
check with its exit code. It reads the diff and reports; it does not edit.

## 4. Report

Front-load the verdict, then three lists, each item with `file:line`:

- **Blockers** — failing checks, missing tests for new behaviour, broken repo rules.
- **Should fix** — convention breaks, unclear names, missing reasons.
- **Notes** — optional polish.

Then: what ran locally (command → exit code), and what is left to CI.
