---
name: crate-persistence
description: Use when editing or adding a SQLx repo impl in crates/persistence (package my-fam-tree-persistence, crate my_fam_tree_persistence) — implementing crate-domain repo traits against Postgres, writing PgUserRepo-style structs, query_as!/query! with FromRow Row structs and column aliases, mapping sqlx errors to FooRepoError, Database::connect/PgPool wiring, or the SERIALIZABLE cycle-check on parent_links.
---

# crate-persistence (my-fam-tree-persistence)

## Overview

Postgres implementations of the repo traits defined in `crate-domain`, backed by
SQLx. **No business logic here** — pure I/O + row↔domain mapping. For the domain
types/traits and invariants see `project-concepts`; for the deny-lint regime, the
`cargo sqlx prepare` / `.sqlx` offline workflow, and the testcontainers harness, see
`rust-foundations` (don't re-derive them). Crate dep direction: `persistence → domain`.

`src/lib.rs` declares one `pub mod` per aggregate and re-exports `Database`,
`PersistenceError`, and every `Pg*Repo` struct.

## Module map

| File | Exports | Notes |
|---|---|---|
| `pool.rs` | `Database` | `connect(url, max_connections, acquire_timeout, statement_timeout_ms)`, `.pool() -> &PgPool`, `.ping()`. Sets `statement_timeout` via `set_config()` in `after_connect`. |
| `error.rs` | `PersistenceError` | `Sqlx(#[from])` + `Config(String)`. **Only** used by `Database`; repos map to domain `FooRepoError` instead. |
| `users.rs` | `PgUserRepo` | reference example; `update_email` maps `users_email_key` → `DuplicateEmail`. |
| `families.rs` `family_memberships.rs` `family_invites.rs` | `PgFamilyRepo` etc. | membership enforces single owner; invites idempotent. |
| `magic_link_tokens.rs` `refresh_tokens.rs` | `PgMagicLinkRepo` `PgRefreshTokenRepo` | single-consume / rotation. |
| `persons.rs` `parent_links.rs` `partnerships.rs` | `PgPersonRepo` etc. | relationship aggregates. |
| `person_contacts.rs` `person_favourites.rs` | | per-person rows. |
| `owner_transfers.rs` `reminder_prefs.rs` `reminder_digests.rs` `audit_log.rs` | | `PgReminderPrefsRepo.get` returns defaults when absent; `upsert` is `ON CONFLICT (user_id) DO UPDATE`. |
| `counts.rs` | `Table`, `count_rows()` | free fn for tests/seeder; returns `sqlx::Error`. |

## Patterns when editing a repo

- Struct holds a `PgPool`: `pub struct PgFooRepo { pool: PgPool }` with
  `pub const fn new(pool: PgPool) -> Self`. Derive `Clone, Debug`.
- `#[async_trait] impl FooRepo for PgFooRepo`. Use **`sqlx::query_as!(Row, …)`** or
  `sqlx::query!` (never free-form `sqlx::query` in non-test code).
- Private `Row` struct (`#[derive(sqlx::FromRow)]`) or the anonymous record `query!`
  produces; `From<Row> for DomainType` + `row.map(Into::into)`. Use column aliases like
  `email::text AS "email!"` / `kind::text AS "kind!"` to force the cast and non-null.
- Pass newtype IDs as `id.into_uuid()`; rebuild with `PersonId::from_uuid(r.id)`.
- Map errors per method: `.map_err(|e| FooRepoError::Db(e.to_string()))`. For unique
  constraints, match `sqlx::Error::Database(db)` on `db.constraint() == Some("…")`
  (e.g. `partnerships_unique_open`) and return the typed variant.
- **parent_links cycle write**: `insert` opens a tx, runs
  `SET TRANSACTION ISOLATION LEVEL SERIALIZABLE`, re-reads edges, runs the in-memory
  `would_create_cycle`, then `INSERT … ON CONFLICT DO NOTHING RETURNING`. This closes
  the TOCTOU window; a DB `BEFORE INSERT` trigger (SQLSTATE `23514`, message
  `parent_links cycle`) is the race-safe backstop, also mapped to `Cycle`.
- **partnerships**: canonicalize the pair to `(lo, hi)` by `into_uuid()` compare so the
  `partner_a_id < partner_b_id` CHECK holds — single statement, **no** SERIALIZABLE tx.

## How to test

- `tests/ping.rs`: skips unless `DATABASE_URL` is set; run via
  `./scripts/cargo-in-network.sh test -p my-fam-tree-persistence --test ping`.
- `tests/auth_repos.rs`: spins a fresh Postgres per test with **testcontainers**
  (`setup()` runs `sqlx::migrate!("../../migrations")`); needs a running Docker daemon.
- Single crate: `cargo test -p my-fam-tree-persistence`.

## Common mistakes

- Adding a query without `rdt sqlx-prepare` → CI sqlx drift (see `rust-foundations`).
- Returning `PersistenceError` from a repo method — return the domain `FooRepoError`.
- Putting validation/business rules here instead of `crate-domain`.
- Forgetting the `::text AS "col!"` alias on enum/nullable columns → wrong inferred type.
