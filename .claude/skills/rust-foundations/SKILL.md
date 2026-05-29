---
name: rust-foundations
description: Use when writing, building, testing, or debugging any Rust crate in my-fam-tree — covers the strict clippy/deny-lint regime (no unwrap/expect/panic/print/indexing), SQLx offline cache + cargo sqlx prepare, the testcontainers integration harness and scripts/cargo-in-network.sh, ApiError/Result conventions, newtype IDs, tracing/logs, and lockfile discipline. Load before touching code under crates/.
---

# Rust Foundations (my-fam-tree workspace)

Cross-cutting backend conventions. For a specific crate's internals, also load its
`crate-<name>` skill. For the domain model, see `project-concepts`. For the
debugging *method*, **REQUIRED BACKGROUND:** superpowers:systematic-debugging; for
new code, superpowers:test-driven-development.

## The strict gate (this trips people up first)

Edition 2024, **nightly** toolchain (`rust-toolchain.toml`). Workspace lints in the
root `Cargo.toml` are not advisory:

- `unsafe_code = forbid`; `unused_must_use = deny`.
- clippy `pedantic` + `nursery` at **deny** (priority -1), plus explicit denies:
  `unwrap_used`, `expect_used`, `panic`, `todo`, `dbg_macro`, `print_stdout`,
  `print_stderr`, `indexing_slicing`.

**Rules that follow from this:**
- No `unwrap` / `expect` / `panic` / `println!` / `eprintln!` / `arr[i]` in non-test
  code. Use `?` + typed errors, `.get(i)`, `tracing::*` macros.
- **Do NOT silence with `#[allow(...)]` in source.** Fix the finding. (`#[cfg(test)]`
  modules and `tests/` files may `#![allow(clippy::unwrap_used, ...)]` — that's the
  only sanctioned place.) If a lint is genuinely wrong, it goes in `Cargo.toml` with a
  rationale, never scattered inline.
- Run the exact CI lint: **`rdt lint`** (or directly
  `cargo +nightly clippy --workspace --all-targets --all-features -- -D warnings -D clippy::pedantic -D clippy::nursery`).
- File size: non-test files **soft 300 / hard 500** lines (the `#[cfg(test)] mod`
  block is stripped before counting `.rs`); dedicated test files hard 500. Enforced by
  `scripts/check-file-size.sh` inside `rdt lint`.

## Crate dependency direction

`domain` depends on nothing internal. `persistence` → `domain`. `cache` and `email`
are standalone. `api` → `{persistence, cache, email, domain}`. `worker` →
same set. `seeder` → `{api, domain, persistence}`. **Never** make `domain` depend on
`persistence`/`api`. Put pure logic + traits in `domain`; put I/O behind a trait and
inject it via `AppState`/`WorkerState` as `Arc<dyn Trait>` — **no global mutable state**.

## Design rules (stay maximally strict)

These are the design choices reviewers expect. Prefer the more constrained option every
time — the deny-lints are the floor, not the whole bar.

- **SQL lives only in `persistence`.** Runtime code (`api`, `worker`,
  `services/`, `domain`) must contain **no** `sqlx::query*` calls or SQL strings — it
  goes through the domain repo traits as `Arc<dyn FooRepo>`. Need a new query? Add the
  method to the trait in `crate-domain`, then implement it in `crate-persistence`. (The
  only other SQL is `migrations/` schema and the `seeder` crate's dev/CI fixtures.)
- **Reuse existing domain structures.** Don't invent parallel structs in
  api/services/worker — use the `crate-domain` types: newtype IDs, `Role`/`Capability`,
  the repo `Row`/`Draft`/result types, `build_upcoming`, `would_create_cycle`,
  `canonicalize_pair`. Map request/response DTOs to/from domain types; one source of truth.
- **Atomic services & functions.** One responsibility each — handlers stay thin and
  delegate orchestration to `crates/api/src/services/`. The 300/500-line ceiling is a
  smell detector, not a target. Any mutation spanning multiple rows/tables, or enforcing
  a read-then-write invariant, must be **atomic**: wrap it in a single transaction
  (SERIALIZABLE where an invariant applies, as `parent_links` cycle-checks do). No
  partial writes.
- **Strictness by construction.** Typed errors over `String`/`anyhow` at boundaries;
  newtypes over bare `Uuid`/`String`; exhaustive `match` (no catch-all `_` that hides a
  new enum variant); validate inputs at the edge; fail loud at startup (config
  validation) instead of limping. Never reach for `#[allow]` to silence a finding.

## Errors & IDs

Every crate owns a precise `thiserror` enum at its boundary; the HTTP layer collapses
all of them into `ApiError` so RFC-7807 stays the single client-visible shape.

| Crate                | Error type        | Bubble-up convention                                            |
|----------------------|-------------------|------------------------------------------------------------------|
| `my-fam-tree-domain`   | `FooRepoError`    | `?` from persistence; mapped in `crate-api` via `impl From`     |
| `my-fam-tree-persistence` | `FooRepoError` | implements the domain traits; surfaces sqlx errors as variants  |
| `my-fam-tree-cache`    | `CacheError`      | `?` from `RateLimiter` / `ReminderJobQueue`; mapped in handlers  |
| `my-fam-tree-email`    | `EmailError`      | `?` from `EmailSender`; mapped in handlers (Internal in prod)   |
| `my-fam-tree-config`   | `ConfigError`     | only at binary startup — `anyhow::Context`, exit non-zero       |
| `my-fam-tree-storage`  | `StorageError`    | `NotFound` → `ApiError::NotFound`, `Backend` → `ApiError::Internal` |

- **Repo / service traits return `Result<T, FooError>`** — `thiserror` enum, never
  `anyhow::Error` at a public trait boundary. Add a named variant for each meaningful
  failure mode; never reach for an `Other(String)` catch-all.
- **Handlers return `Result<ApiResponse<T>, ApiError>`.** The mapping from a
  crate-specific error → `ApiError` lives in `impl From<FooError> for ApiError`
  inside `crates/api/src/error.rs`. New errors gain an entry there, never inline
  `match` ladders inside a route.
- **`HttpResponse::` is forbidden** outside `crates/api/src/response.rs`. The
  `ApiResponse<T>` + `ApiError` pair is the only legal contract.
- **`anyhow::Context` is for binary entry-points only** — `bin/api.rs`, `bin/worker`,
  `bin/seeder`. Library code uses the typed boundary errors.
- **Never `.expect()` / `.unwrap()`** — workspace clippy denies both. `?` + a named
  error variant or a `match` arm that returns a precise `ApiError`.

IDs are newtypes (`UserId`, `FamilyId`, `PersonId`, …) via the `id_newtype!` macro —
`from_uuid` / `into_uuid` / `as_uuid`, transparent serde. Pass the newtype, not a
bare `Uuid`, so the type system catches mix-ups.

## SQLx (compile-checked queries + offline cache)

- Use **`sqlx::query!` / `sqlx::query_as!`** only. Free-form `sqlx::query` is denied by
  a CI grep.
- Queries are verified at compile time against `.sqlx/` (committed offline cache).
- After adding/changing any query: run **`rdt sqlx-prepare`**
  (`cargo sqlx prepare --workspace`) against a migrated DB
  (`DATABASE_URL=postgres://my_fam_tree:my_fam_tree@localhost:3458/my_fam_tree`), then commit
  the `.sqlx/` diff **in the same commit as the query change**.
- `SQLX_OFFLINE=true` builds without a DB (CI default).
- Migrations live in `migrations/` (additive, numbered `NNNN_*.sql`); apply with
  `rdt migrate`, check drift with `rdt migrate-check`.

## Testing & debugging

- Integration tests live in `crates/<crate>/tests/*.rs`. The API suite uses an
  **ephemeral testcontainers stack** (`ephemeral_stack()` in `crates/api/tests/common/mod.rs`):
  it boots throwaway Postgres + Redis, runs migrations, builds `AppState` with a
  `FakeEmailSender`, and exposes `build_app(state, None)` + helpers `sign_in`,
  `create_family`, `try_call`, `extract_token_from_link`. **Needs a running Docker
  daemon**, not the full compose stack.
- A few tests hit live infra and **skip unless env vars are set** (e.g.
  `crates/email/tests/mailpit.rs` needs `EMAIL_DSN` + `MAILPIT_API`). Run those in the
  compose network: `./scripts/cargo-in-network.sh test -p my-fam-tree-email --test mailpit`.
- Run everything: `cargo test --workspace` (or `rdt test`, which also runs FE tests).
- One file, with output: `cargo test -p my-fam-tree-api --test auth_flow -- --nocapture`.
- Logs: services emit `tracing` (`RUST_LOG=info,my_fam_tree=debug`, `LOG_FORMAT=pretty`
  in dev / `json` in prod). Read them with `docker compose logs -f api` /
  `… worker`. Set `RUST_BACKTRACE=1` when chasing a panic in a binary.

## Common mistakes

| Symptom | Cause / fix |
|---|---|
| clippy fails on `unwrap`/`expect`/index | Rewrite with `?` + `.get()`; don't `#[allow]` it. |
| `error: `DATABASE_URL` must be set` at build | Build offline: `SQLX_OFFLINE=true`, or point at the dev DB on `:3458`. |
| query change won't compile / CI "sqlx drift" | Re-run `rdt sqlx-prepare` and commit `.sqlx/` with the change. |
| `cargo build --locked` fails in CI | You changed `Cargo.toml` without committing the `Cargo.lock` diff together. |
| integration test hangs/errors connecting | Docker daemon not running (testcontainers), or the test needs `cargo-in-network.sh`. |
| `HttpResponse` won't pass review | Return `Result<ApiResponse<T>, ApiError>`; only `response.rs` builds raw responses. |
