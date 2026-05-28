---
name: project-concepts
description: Use when working anywhere in the my-family repo and you need the lay of the land — the domain model (families, persons, parent-links, partnerships, contacts, reminders), roles and capabilities, the magic-link + JWT auth flow, the API response/error envelope, or how the compose services fit together. Load this first when orienting in an unfamiliar area.
---

# my-family — Project Concepts

## Overview

my-family is a self-hosted platform where users create **families**, collaboratively
maintain a **family tree**, share **contact data**, and receive configurable
**birthday-reminder** emails. Rust backend (Actix-web + SQLx + Postgres + Redis),
Vue 3 frontend, magic-link (passwordless) auth.

This skill is the orientation map. For *how to build/test* a stack, load
`rust-foundations` (backend) or `frontend-workflow` (FE). For a specific crate, load
its `crate-<name>` skill.

## Service topology

The whole stack runs in Docker Compose (`compose.yaml`). Locally, **dinghy** routes
`*.my-family.docker` to the right container; only Postgres binds a host port.

| Service | URL / address | Notes |
|---|---|---|
| FE (Vite) | http://my-family.docker | dinghy → `fe:5173`; proxies `/api/*` → `api:8080` |
| API | http://api.my-family.docker | Swagger UI at `/api/docs`; internal `:8080` |
| Mailpit | http://mail.my-family.docker | dev inbox UI (`:8025`); SMTP internal `:1025` |
| worker | `worker.my-family.docker` | metrics/test listener `:9091` |
| Postgres | `localhost:3458` (host) → `5432` | db/user/pass all `my_family` |
| Redis | `redis:6379` (internal only) | key prefix `my-family:` |
| Playwright | in-network only | `E2E_BASE_URL=http://my-family.docker:5173` |

**Lifecycle:** `rdt start` / `rdt stop` (or `docker compose up -d` / `down`).
Project commands live in `.rusty_dev_tool/config.toml`: `migrate`, `migrate-status`,
`migrate-check`, `sqlx-prepare`, `openapi`, `openapi-check`, `lint`, `test`,
`test-e2e`, `coverage`, `fe`, `worker`, `gen-jwt-keys`, `seed`, `reset`, `deny`.
**Logs:** `docker compose logs -f <service>` (e.g. `api`, `worker`).

## Domain model

A **user** is a login identity (`email`, `locale`). A **family** is one tree /
workspace. Membership is the `(user, family, role)` join — a user can belong to many
families with a different role in each.

| Entity | Meaning |
|---|---|
| `users` | login identity; passwordless |
| `families` | a tree / workspace |
| `family_memberships` | `(user, family, role)`; role ∈ `user` / `admin` / `owner` |
| `family_invites` | pending invite (token, role, optional `person_id` to claim) |
| `magic_link_tokens` | sign-in + email-change tokens (`MagicLinkPurpose`) |
| `refresh_tokens` | rotating JWT refresh records |
| `persons` | a node in the tree; need NOT map to a user |
| `parent_links` | directed `(child → parent)` edge, `ParentKind`; **cycle-prevented** |
| `partnerships` | undirected partner pair, stored canonical `a<b`; `PartnershipKind` |
| `person_contacts` | contact rows on a person; `ContactKind` + `ContactVisibility` |
| `person_favourites` | a user's starred persons |
| `owner_transfers` | two-sided ownership handover (`TransferSide`) |
| `reminder_preferences` | per-user digest prefs (timezone, lead time) |
| `reminder_digests` | scheduled/sent digest rows (`DigestStatus`) |
| `audit_log` | append-only log of mutating actions |

Relationship invariants (in `crate-domain`): parent-links are acyclic
(`would_create_cycle`); partner pairs are canonicalized to `(min, max)` so the DB
`CHECK (partner_a_id < partner_b_id)` holds (`canonicalize_pair`).

## Roles & capabilities

Three roles, ranked **`user` < `admin` < `owner`** (`Role::at_least`). Authorization
checks a `Capability`, not a role directly (`capabilities_of` / `has`):

- **user** — self-service only: edit own person, own contacts, own reminders.
- **admin** — all user caps + create/edit/delete persons, manage relationships, edit
  any contact, invite users, manage roles below owner.
- **owner** — all admin caps + transfer ownership + delete family. Exactly one owner.

## Auth model

Passwordless. Request a magic link → email (Mailpit in dev) → consume the token →
the API sets **access** + **refresh** cookies. JWTs are **Ed25519** (PEM keys, `kid`
rotation) and carry a **`families[]` claim** listing the user's memberships + roles.

Per request, the FE sends the active family in the **`X-Family-Id`** header;
`AuthMiddleware` cross-references it against the `families[]` claim. Cookies:
access = `SameSite=Lax`, refresh = `SameSite=Strict`, `COOKIE_DOMAIN=.my-family.docker`
(`COOKIE_SECURE=false` in dev). Magic-link issuance is rate-limited per email + per IP.

## API envelope & error model

**Success:** every handler returns `Result<ApiResponse<T>, ApiError>`.
`ApiResponse<T> = { data, meta, warnings? }`. DELETE returns `{ "data": null }`
(never `204`, never `{ "status": "deleted" }`).

**Errors:** RFC-7807 `application/problem+json`:
`{ type, title, status, code, detail, instance, fields }`. `code` is a stable
machine string (`ErrorCode`, e.g. `auth.unauthenticated`, `family.not_member`);
`fields[]` carries per-field violations for `422`. The FE maps `code` → i18n message.

## The API type contract (OpenAPI → TS)

The backend is the single source of truth for the API types; the FE consumes generated
TypeScript. The pipeline spans **both** stacks — know all of it:

1. **Rust annotations** — each handler carries `#[utoipa::path(...)]` + schema derives;
   `crates/api/src/openapi_doc.rs` aggregates them into `ApiDoc` (see `crate-api`).
2. **Dump** — the `crates/openapi` `openapi-dump` binary serializes `ApiDoc` to the
   **committed** `fe/openapi.json` (see `crate-ops-binaries`).
3. **Codegen** — `openapi-typescript` turns `fe/openapi.json` into
   `fe/src/api/schema.d.ts` (**gitignored, regenerated, never hand-edited**), consumed by
   the `openapi-fetch` client and re-exported via `fe/src/api/types.ts` (see
   `frontend-workflow`).

**Regenerate both** with `rdt openapi` (dump → `fe/openapi.json`, then codegen →
`schema.d.ts`). `rdt openapi-check` diffs a fresh dump against the committed
`fe/openapi.json` and fails on drift (CI + pre-push gate).

**Adding/changing API content:** edit the Rust endpoint → `rdt openapi` → **commit the
updated `fe/openapi.json`** → consume the new types from a FE hook. Never hand-edit
`fe/openapi.json` or `schema.d.ts`.

## Where things live

```
crates/domain         pure types, newtype IDs, repo traits, roles/capabilities (no I/O)
crates/persistence    SQLx Postgres impls of the repo traits (.sqlx offline cache)
crates/cache          Redis pool, rate limiter, reminder job queue
crates/email          SMTP + Fake senders, Askama text templates (en/de)
crates/api            Actix HTTP server: routes, AppState, middleware, error model, OpenAPI
crates/worker leader-locked scheduler + dispatcher pool for digest emails
crates/migrator       sqlx migrate runner binary
crates/seeder         deterministic dev/CI seed binary
crates/openapi        openapi-dump binary (spec → fe/openapi.json)
migrations/           0001..NNNN SQL migrations (additive)
fe/src                Vue 3 app (api client, components, views, stores, router, i18n)
fe/e2e                Playwright E2E (fixtures, page objects, global setup/teardown)
```
