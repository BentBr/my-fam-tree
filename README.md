# my-family

[![CI](https://github.com/BentBr/my-family/actions/workflows/ci.yml/badge.svg)](https://github.com/BentBr/my-family/actions/workflows/ci.yml)
[![Release Please](https://github.com/BentBr/my-family/actions/workflows/release-please.yml/badge.svg)](https://github.com/BentBr/my-family/actions/workflows/release-please.yml)
[![Latest release](https://img.shields.io/github/v/release/BentBr/my-family?display_name=tag&sort=semver&label=release)](https://github.com/BentBr/my-family/releases/latest)
[![License: BUSL-1.1](https://img.shields.io/badge/license-BUSL--1.1-blue.svg)](LICENSE)
[![Rust nightly](https://img.shields.io/badge/rust-nightly-orange.svg)](rust-toolchain.toml)
[![BE coverage](https://img.shields.io/endpoint?url=https://raw.githubusercontent.com/BentBr/my-family/gh-pages/coverage.json)](https://github.com/BentBr/my-family/actions/workflows/ci.yml)
[![FE coverage](https://img.shields.io/endpoint?url=https://raw.githubusercontent.com/BentBr/my-family/gh-pages/coverage-fe.json)](https://github.com/BentBr/my-family/actions/workflows/ci.yml)

A platform for managing family trees, contact data, and birthday reminders. Built with Rust + PostgreSQL + Redis + Vue 3.

**License:** [BUSL-1.1](./LICENSE) · **Plain-English:** [LICENSING.md](./LICENSING.md)

## Quick start

```bash
# 1. Host prerequisites — Rust + Docker + the dev toolbelt.
#    Rust nightly auto-installs via rust-toolchain.toml on first build.
#    Docker Desktop (or daemon). No host-side Node or pnpm install required;
#    everything FE runs in the `fe` compose container.
cargo install sqlx-cli cargo-llvm-cov cargo-deny cargo-machete

# 2. dinghy for *.docker subdomain routing (macOS).

# Install Dinghy (just another docker container) - for MAC OS:
docker run -d --restart=always \
   -v /var/run/docker.sock:/tmp/docker.sock:ro \
   -v ~/.dinghy/certs:/etc/nginx/certs \
   -p 80:80 -p 443:443 -p 19322:19322/udp \
   -e CONTAINER_NAME=http-proxy \
   --name http-proxy \
   codekitchen/dinghy-http-proxy

# Setup the resolver
sudo mkdir -pv /etc/resolver
sudo bash -c 'echo "nameserver 127.0.0.1" > /etc/resolver/docker'
sudo bash -c 'echo "port 19322" >> /etc/resolver/docker'


# 3. Install git hooks (commit-msg + pre-push). Run once per checkout.
./scripts/install-hooks.sh

# 4. Bootstrap .env — copy template + append fresh Ed25519 JWT keys.
cp .env.example .env
cargo run -p my-family-api --bin gen-jwt-keys >> .env

# 5. Bring up the full stack (postgres, redis, mailpit, migrator, api, worker, fe).
#    The fe container runs pnpm install on first boot — no host pnpm needed.
rdt start

# 6. Regenerate FE OpenAPI types from the live spec.
rdt openapi
```

Open **http://my-family.docker** in your browser — you should see the health page reporting API status. Other endpoints:

- API:     http://api.my-family.docker (Swagger at `/api/docs`)
- Mailpit: http://mail.my-family.docker
- Postgres: `psql -h localhost -p 3458 -U my_family my_family` (or point your IDE at port 3458)

The `seeder` container runs once after `migrator` completes and prints three `MAGIC_LINK` lines — one per seeded user (admin / alice / bob). View them with `docker compose logs seeder` (or `rdt seed` to re-run; the seed is idempotent) and paste the URL into the browser to sign in.

> Linux / no-dinghy local dev support is on the roadmap; for now the local stack assumes dinghy is installed (macOS one-liner above). CI uses ephemeral GitHub Actions services and does not depend on dinghy.

### Troubleshooting first boot

| Symptom | Likely cause | Fix |
|---|---|---|
| `http://my-family.docker` does not resolve | dinghy not running / `/etc/resolver/docker` missing | `dinghy up && dinghy status` |
| `compose up` errors with `JWT_PRIVATE_KEY required` | Step 4 was skipped | Run `gen-jwt-keys >> .env` |
| `pre-push` hook never runs | Step 3 was skipped | `./scripts/install-hooks.sh` |
| API rejects every token with `auth_token_invalid` | Placeholder JWT_* lines bled into `.env` | Remove `JWT_PRIVATE_KEY*` / `JWT_PUBLIC_KEYS` from `.env`, re-run `gen-jwt-keys` |
| IDE can't reach Postgres | Wrong port | Use host `localhost`, port `3458` (NOT 5432), user/db = `my_family` |
| `pnpm` complains about `node_modules` from a different major | Stale FE deps from a previous toolchain | Set `CI=true` in container env (already in compose) or `rm -rf fe/node_modules fe/.pnpm-store` and re-run `rdt start` |

## Stack

- **Backend:** Rust (edition 2024, nightly toolchain) · Actix-web 4 · SQLx 0.8 · PostgreSQL 18 · Redis 7
- **Frontend:** Vue 3.5 · TypeScript 6 (strict) · Vite 8 · Pinia 3 · vue-router 5 · Vuetify 4 · vue-i18n 11 · TanStack Query 5 · openapi-fetch 0.17 · ESLint 10
- **Tooling:** [RDT](https://github.com/BentBr/rusty_dev_tool) for quick commands · pnpm 10.33.4 (container-only via `scripts/fe-in-container.sh`) · Docker Compose for local infra · GitHub Actions for CI · Playwright for E2E

## Project layout

```
crates/         Rust workspace (api, reminder-worker, migrator, domain, persistence, email, cache, openapi)
fe/             Vue 3 + TS frontend (all pnpm runs via scripts/fe-in-container.sh)
migrations/     SQLx migrations
.docker/        Per-service Dockerfiles
.github/        Actions workflows, Dependabot, PR template
.githooks/      Pre-push and commit-msg hooks
scripts/        Helper scripts (file-size check, hooks installer, fe-in-container wrapper)
compose.yaml    Postgres, Redis, Mailpit, api, worker, migrator, fe
.dockerignore   Build-context exclusions
.env.example    Every supported env var with comments
```

## Common commands (via `rdt`)

| Command | What it does |
|---|---|
| `rdt start` | `docker compose up -d` (fe container auto-installs deps on first boot) |
| `rdt stop` | `docker compose down` |
| `rdt shell` | Open a shell in the api container |
| `rdt migrate` / `rdt m` | Apply pending DB migrations |
| `rdt migrate-status` | Show applied vs pending |
| `rdt openapi` | Dump OpenAPI spec + regenerate FE types (routed through the fe container) |
| `rdt openapi-check` | Fail if the committed spec drifted |
| `rdt lint` / `rdt l` | All linters (rust + fe + file-size); FE checks run inside the fe container |
| `rdt test` / `rdt t` | All unit tests |
| `rdt test-e2e` | Playwright E2E against the running stack |
| `rdt coverage` | Coverage report |
| `rdt gen-jwt-keys` | Print a fresh Ed25519 keypair for .env |
| `rdt seed` | Run the deterministic seed (idempotent UPSERTs; prints MAGIC_LINK URLs) |
| `rdt reset` | Drop dev DB volume, re-migrate, and re-seed |

Run `rdt help` for the full list. FE commands (`pnpm lint`, `pnpm test`, codegen) are always dispatched via `scripts/fe-in-container.sh` — you never invoke pnpm on the host.

## Testing

- **Rust:** `cargo test --workspace`. Integration tests use real Postgres/Redis from compose.
- **Frontend component tests:** `pnpm test` (Vitest) — runs inside the fe container.
- **End-to-end:** `pnpm test:e2e` (Playwright) against the live stack. Asserts the API + FE + Mailpit interactions.
- **Coverage thresholds:** backend ≥80% (crates/domain target ≥85%), `fe/src/` ≥80%. Enforced in CI.

## Strictness

- **Rust:** clippy `pedantic` + `nursery` (warn), `unwrap_used` / `expect_used` / `panic` / `todo` / `print_*` / `indexing_slicing` denied. Newtype IDs everywhere. `sqlx::query!` only (committed `.sqlx/`).
- **TypeScript:** strict mode + `noUncheckedIndexedAccess` + `exactOptionalPropertyTypes` + `verbatimModuleSyntax`. ESLint forbids `any` and non-null assertions. Branded ID types.
- **File-size limits:** non-test code soft 300 / hard 500 lines (test module excluded); test files hard 500. Enforced by `scripts/check-file-size.sh` in `rdt lint`.
- **Commits:** Conventional Commits, enforced by `.githooks/commit-msg`.

## Contributing

See [CONTRIBUTING.md](./CONTRIBUTING.md). Security disclosures: [SECURITY.md](./SECURITY.md).

## License

BUSL-1.1 — see [LICENSE](./LICENSE) and the plain-English [LICENSING.md](./LICENSING.md). Self-hosting for personal use is explicitly permitted; commercial hosted offerings are not.
