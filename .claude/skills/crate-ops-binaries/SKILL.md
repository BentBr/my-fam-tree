---
name: crate-ops-binaries
description: Use when running, debugging, or modifying the three thin operational binaries — migrator, seeder, openapi — or their rdt commands. Triggers only. Keywords: run_migrations, sqlx migrate, seed, MAGIC_LINK, openapi-dump, openapi spec, ApiDoc, migrate-status, migrate-check, drift.
---

# Crate Ops Binaries (migrator / seeder / openapi)

Three tiny single-purpose binary crates supporting DB lifecycle, dev data, and
the OpenAPI spec. For the strict lint regime and SQLx rules see `rust-foundations`;
for the domain model and `rdt` topology see `project-concepts`. Each `rdt`
command is just the `execution` string in `.rusty_dev_tool/config.toml`.

## migrator — apply/inspect SQLx migrations

Package `my-fam-tree-migrator`, bin `run_migrations`. Wraps
`sqlx::migrate!("../../migrations")` and applies the numbered SQL in `migrations/`
into the `_sqlx_migrations` table (name pinned via `MIGRATIONS_TABLE` const for
sqlx 0.9 parity). Key file: `crates/migrator/src/main.rs`.

Flags (clap): `--status` (list APPLIED/PENDING then exit), `--check`
(**CI gate** — `eprintln` + `std::process::exit(2)` if any pending, applies
nothing), `--dry-run` (print `WOULD APPLY …`), `--target N` (apply up to and
including version N), `--database-url` (env `DATABASE_URL`). No flags = apply all.

Commands: `rdt migrate` = `docker compose run --rm migrator`; `rdt migrate-status`
= `… migrator --status`; `rdt migrate-check` = `… migrator --check`. Other
services `depends_on` migrator `service_completed_successfully`.

## seeder — deterministic dev/CI seed

Package `my-fam-tree-seeder` (lib + bin `seed`). UPSERTs a fixed dataset (3 users,
1 family `Müller`, 22 persons, 22 parent-links, 8 partnerships, 9 contacts) with
`ON CONFLICT … DO UPDATE`, so re-running is a no-op on row counts. Every UUID is
hardcoded in `crates/seeder/src/ids.rs` (structured hex blocks: `…0001…` users,
`…0002…` family, `…0003…` persons, `…0004…` partnerships) for idempotency,
including closed partnership rows. Orchestrator: `crates/seeder/src/lib.rs`.

It mints one fresh login magic link per user via the shared `mint_magic_link_url`
and prints grep-friendly `MAGIC_LINK <email> <url>` lines (`src/main.rs`) — paste
into the browser to sign in as admin / alice / bob. **Separate crate** depending
on `{api, domain, persistence}`, built from `.docker/seeder.Dockerfile`, so prod
api/worker images never ship a seed-capable binary.

Commands: `rdt seed` = `docker compose run --rm seeder`; `rdt reset` drops
volumes then re-migrates + re-seeds.

## openapi — dump the spec

Package `my-fam-tree-openapi`, bin `openapi-dump`. `src/lib.rs` just re-exports
`my_fam_tree_api::ApiDoc` (definition lives in `crates/api/src/openapi_doc.rs` to
avoid a circular dep). `src/bin/openapi_dump.rs` prints
`ApiDoc::with_cookie_auth()` as pretty JSON to stdout — it carries
`#[allow(clippy::print_stdout, reason = …)]` because the workspace denies
`print_stdout`.

`rdt openapi` = `cargo run -p my-fam-tree-openapi --bin openapi-dump > fe/openapi.json
&& ./scripts/fe-in-container.sh openapi-codegen`. `rdt openapi-check` diffs the
fresh dump against the committed `fe/openapi.json` to catch drift.

## Common mistakes

| Symptom | Fix |
|---|---|
| Changed an API endpoint, FE types stale / CI drift fails | Re-run `rdt openapi`; commit `fe/openapi.json` + `fe/src/api/schema.d.ts`. |
| `migrate-check` exits 2 in CI | A migration is unapplied — run `rdt migrate` against the target DB. |
| Added seed rows but tests fail counts | Update `SEED_PERSON_COUNT` / `EXPECTED_*` consts in `ids.rs` + lib tests. |
| Adding `print_stdout` elsewhere expecting it to pass | These three binaries are the *only* sanctioned `print_stdout` allows; don't copy the pattern into library code. |
