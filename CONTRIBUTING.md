# Contributing to my-family

Thanks for your interest. A few short conventions keep the project consistent.

## Setup

1. Install host prerequisites: Rust (nightly per `rust-toolchain.toml`) and Docker (Desktop or daemon). **No host-side Node or pnpm install is required** — all FE tooling runs inside the `fe` compose container, dispatched via `scripts/fe-in-container.sh`.
2. Install tooling:

    ```bash
    cargo install rusty-dev-tool sqlx-cli cargo-llvm-cov cargo-deny cargo-machete
    ```

3. Install git hooks: `./scripts/install-hooks.sh`.
4. Copy env: `cp .env.example .env`, then append fresh JWT keys: `cargo run -p my-family-api --bin gen-jwt-keys >> .env`.
5. Bring up the stack: `rdt start`. The `fe` container runs `pnpm install` on first boot — no host pnpm needed.

## Workflow

- Branch off `main`. One concern per PR.
- Write tests first when feasible (TDD is opt-in but encouraged for `crates/domain`).
- Run `rdt lint && rdt test` locally before opening a PR. Both commands route FE checks through the container wrapper transparently.
- Pre-push hook runs the same checks (skippable via `.env.local` for individual checks; never skip the whole hook in PRs).

## Commit messages

Conventional Commits, enforced by `.githooks/commit-msg`:

- `feat:`, `fix:`, `chore:`, `refactor:`, `test:`, `docs:`, `ci:`, `build:`, `perf:`, `style:`
- Optional scope: `feat(tree): collapse distant branches`
- Subject ≤ 72 chars, imperative ("add", not "added").
- No `Co-Authored-By` trailers.

## Coding standards

See the README's *Strictness* section. In short:

- No `unwrap`/`expect`/`panic` outside tests and binary `main`.
- No `any` on the FE; no non-null assertions; no broad `as` casts.
- Files ≤ 500 lines (non-test code excludes the in-file test module); split before you reach 300.
- Every endpoint returns `Result<ApiResponse<T>, ApiError>`. Never construct `HttpResponse` outside `crates/api/src/response.rs`.

## Tests

- Cargo unit tests live next to the code in `#[cfg(test)] mod tests`.
- Cargo integration tests live in `crates/*/tests/`.
- FE component tests live under `fe/tests/` (Vitest).
- E2E tests live in `fe/e2e/tests/` (Playwright against the live stack).
- Coverage thresholds are enforced in CI; see README.

## Opening a PR

Fill out the PR template. Link any related issue. Add screenshots for UI changes. Mention any DB migration impact.

## Reporting security issues

See [SECURITY.md](./SECURITY.md). Do not file public issues for vulnerabilities.
