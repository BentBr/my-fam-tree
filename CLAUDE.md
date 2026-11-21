# my-fam-tree

A platform where families maintain a shared family tree, share contact data and get
birthday-reminder emails. Rust workspace (Actix-web, SQLx, Postgres, Redis) + Vue 3 / Vuetify
frontend. Start with the `project-concepts` skill for the domain model, auth flow and how the
compose services fit together.

## Running things

- `rdt` is the task runner — see the README tables for every command and helper script.
  `rdt start` is the documented way to bring the stack up; don't replace it with raw compose.
- Host `cargo` is fine. Tests that need compose hostnames (postgres/redis/mailpit) run inside
  the network: `./scripts/cargo-in-network.sh test -p <package> --test <name>`.
- Host builds without a reachable DB: `SQLX_OFFLINE=true` (CI default; `.sqlx/` is committed).
  Changed a `query!`? Run `rdt sqlx-prepare` and commit `.sqlx/`.
- Frontend tooling runs only in the container: `./scripts/fe-in-container.sh <pnpm-script>`.
  Never run node/pnpm on the host.
- OpenAPI: `rdt openapi` regenerates `fe/openapi.json` + TS types, `rdt openapi-check` fails on
  drift. Both go through `scripts/openapi.sh`.

## Gates

- pre-push runs the full local suite (fmt, clippy, tests, FE lint/typecheck/tests/knip, openapi
  drift, machete, file size). That is intended. Never `--no-verify`.
- Clippy gate: `cargo +nightly clippy --workspace --all-targets --all-features -- -D clippy::all
  -D warnings -D clippy::pedantic -D clippy::nursery`.
- Honest exit codes: never `cmd | tail && echo OK`. Use `set -o pipefail`, or capture `rc=$?`
  right after the command and exit with it.
- `cargo test -p my-fam-tree-arch-tests` enforces repo rules (below). Read its failure message —
  it names the file and the broken rule.

## Code rules

- Workspace lints deny unwrap/expect/panic/indexing/print outside tests, plus pedantic + nursery
  and a measured set (see `[workspace.lints]` in `Cargo.toml`). Test allowances live in
  `clippy.toml`.
- Suppress a lint only with `#[expect(lint, reason = "…")]` — `#[allow]` is denied. Structural
  causes get one module-level `#![expect]` (e.g. actix handler modules).
- Crate boundaries live in `crates/arch-tests/src/rules.rs`. Only `persistence` talks to the
  database; all SQL lives there (the seeder's statements are in `persistence::seed`). Change a
  rule deliberately with a reason — never add an exception just to get green.
- Every `scripts/` file and every rdt command is documented in the README; every workflow job has
  `timeout-minutes`; paths quoted in skills must exist. All enforced by arch-tests.
- File size: source soft 300 / hard 500 lines (test module excluded), test files hard 500.
- Every code change ships with tests: main path plus important edge/error cases; a bug fix gets a
  test that fails without the fix.

## Known traps

- Actix: two `web::scope("/api/v1")` on one `App` shadow each other — nest empty-path
  sub-scopes for middleware grouping instead.
- Actix middleware: never hold an extra `HttpRequest` clone across `svc.call(req)` (routing
  panics); return `Err(Error::from(ApiError))`.
- `build_app` returns `ServiceResponse<impl MessageBody>` (not `BoxBody`) and needs `+ use<>`
  precise captures under edition 2024.

## Working with Bent

- When Bent edits a file, that edit is deliberate. Don't revert it — ask first.
- Conventional Commits (`type(scope): subject`, enforced by `.githooks/commit-msg`); branches
  `type/kebab-name`. No `Co-Authored-By` or other AI attribution in commits or PRs.
- Use the latest stable dependencies; after a major bump verify APIs instead of trusting old
  examples.

## Delegating to subagents

- One editing target per agent; say explicitly "no Co-Authored-By trailers".
- Verify every claim against the real gate (clippy / tests / CI), not against the agent's
  summary. A silent no-op is a failure.
- Findings go in the agent's final message — no report files.

## Skills

- `project-concepts` — domain model, roles, auth flow, envelope, compose services.
- `rust-foundations` — lint regime, SQLx offline, integration-test harness, errors, IDs.
- `crate-api` — endpoints, AppState, auth middleware, ApiError, OpenAPI.
- `crate-domain` — repo traits, newtype IDs, roles/capabilities, relationship invariants.
- `crate-persistence` — SQLx repo impls, pool wiring, seed statements.
- `crate-cache` · `crate-email` · `crate-storage` · `crate-config` · `crate-worker` — one per crate.
- `crate-ops-binaries` — migrator, seeder, openapi binaries and their rdt commands.
- `frontend-workflow` — Vue/Vuetify, stores, API client, i18n, container-only tooling.
- `playwright-e2e` — E2E tests and live browser debugging.
- `/qa` (user-invoked) — scope the change, run targeted gates, read-only review by the
  `quality-assurance` agent. `/qa delta` checks only what is new since the last push.
