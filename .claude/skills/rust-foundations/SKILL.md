---
name: rust-foundations
description: Use when writing, building, testing, or debugging any Rust crate in my-family — covers the strict clippy/deny-lint regime (no unwrap/expect/panic/print/indexing), SQLx offline cache + cargo sqlx prepare, the testcontainers integration harness and scripts/cargo-in-network.sh, ApiError/Result conventions, newtype IDs, tracing/logs, and lockfile discipline. Load before touching code under crates/.
---

# Rust Foundations (my-family workspace)

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
are standalone. `api` → `{persistence, cache, email, domain}`. `reminder-worker` →
same set. `seeder` → `{api, domain, persistence}`. **Never** make `domain` depend on
`persistence`/`api`. Put pure logic + traits in `domain`; put I/O behind a trait and
inject it via `AppState`/`WorkerState` as `Arc<dyn Trait>` — **no global mutable state**.

## Errors & IDs

- Repo traits return `Result<T, FooRepoError>` (`thiserror` enums in `crate-domain`).
- The HTTP layer maps everything to `ApiError` → RFC-7807 (see `crate-api`). Handlers
  return `Result<ApiResponse<T>, ApiError>`; `HttpResponse::` is forbidden outside
  `crates/api/src/response.rs`.
- IDs are newtypes (`UserId`, `FamilyId`, `PersonId`, …) via the `id_newtype!` macro —
  `from_uuid` / `into_uuid` / `as_uuid`, transparent serde. Pass the newtype, not a
  bare `Uuid`, so the type system catches mix-ups.

## SQLx (compile-checked queries + offline cache)

- Use **`sqlx::query!` / `sqlx::query_as!`** only. Free-form `sqlx::query` is denied by
  a CI grep.
- Queries are verified at compile time against `.sqlx/` (committed offline cache).
- After adding/changing any query: run **`rdt sqlx-prepare`**
  (`cargo sqlx prepare --workspace`) against a migrated DB
  (`DATABASE_URL=postgres://my_family:my_family@localhost:3458/my_family`), then commit
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
  compose network: `./scripts/cargo-in-network.sh test -p my-family-email --test mailpit`.
- Run everything: `cargo test --workspace` (or `rdt test`, which also runs FE tests).
- One file, with output: `cargo test -p my-family-api --test auth_flow -- --nocapture`.
- Logs: services emit `tracing` (`RUST_LOG=info,my_family=debug`, `LOG_FORMAT=pretty`
  in dev / `json` in prod). Read them with `docker compose logs -f api` /
  `… reminder-worker`. Set `RUST_BACKTRACE=1` when chasing a panic in a binary.

## Common mistakes

| Symptom | Cause / fix |
|---|---|
| clippy fails on `unwrap`/`expect`/index | Rewrite with `?` + `.get()`; don't `#[allow]` it. |
| `error: `DATABASE_URL` must be set` at build | Build offline: `SQLX_OFFLINE=true`, or point at the dev DB on `:3458`. |
| query change won't compile / CI "sqlx drift" | Re-run `rdt sqlx-prepare` and commit `.sqlx/` with the change. |
| `cargo build --locked` fails in CI | You changed `Cargo.toml` without committing the `Cargo.lock` diff together. |
| integration test hangs/errors connecting | Docker daemon not running (testcontainers), or the test needs `cargo-in-network.sh`. |
| `HttpResponse` won't pass review | Return `Result<ApiResponse<T>, ApiError>`; only `response.rs` builds raw responses. |
