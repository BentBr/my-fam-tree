---
name: crate-domain
description: Use when touching the my-family-domain crate (crate `my_family_domain`, under crates/domain) — adding/changing a repo trait or its `FooRepoError`/Row/Draft types, working with newtype IDs (UserId/FamilyId/PersonId/FamilyMembershipId, `id_newtype!`, from_uuid/into_uuid/as_uuid), `Role`/`Capability`/`capabilities_of`/`has`/`at_least`, the relationship invariants `would_create_cycle`/`canonicalize_pair`, or the `build_upcoming`/UpcomingEvent/UpcomingFilter projection. Symptoms: where do I define a repo method, why is domain pure, exhaustive Capability match.
---

# crate-domain (`my-family-domain`)

The dependency root: **pure types, newtype IDs, repo traits, role/capability
logic, relationship invariants**. No I/O — no `tokio`, `sqlx`, or Actix in
`Cargo.toml`. Repo traits are `async` (via `async-trait`) but their Postgres
impls live in `crate-persistence`. For the domain model see `project-concepts`;
for the strict-lint/test regime see `rust-foundations`.

`src/lib.rs` re-exports everything (`use my_family_domain::PersonRepo;`).

## Module map

| File | Responsibility |
|---|---|
| `src/ids.rs` | `id_newtype!` macro → `UserId`, `FamilyId`, `PersonId`, `FamilyMembershipId`. Transparent serde over `Uuid`; `from_uuid`/`into_uuid`/`as_uuid`. |
| `src/role.rs` | `Role` enum `User < Admin < Owner` (snake_case serde); `Role::at_least(needed)`. |
| `src/capabilities.rs` | `Capability` enum + `capabilities_of(role) -> &'static [Capability]` + `has(role, cap)`. |
| `src/relationships.rs` | `would_create_cycle(edges, child, parent)`, `canonicalize_pair(a, b) -> Option<(min, max)>`. Pure, no DB. |
| `src/upcoming.rs` | `build_upcoming(...)`, `UpcomingEvent`, `UpcomingKind`, `UpcomingFilter`, `DEFAULT_LIMIT`/`MAX_LIMIT`. |
| `src/repos/*.rs` | One trait + `FooRepoError` + Row/Draft types per aggregate. |

`repos/`: `users`, `families`, `family_memberships`, `family_invites`,
`magic_link_tokens`, `refresh_tokens`, `persons`, `parent_links`,
`partnerships`, `person_contacts`, `person_favourites`, `owner_transfers`,
`reminder_prefs`, `reminder_digests`, `audit_log`.

## Key invariants

- **IDs are distinct types** even though serde is transparent — pass the
  newtype, never a bare `Uuid`, so the compiler catches `UserId`/`FamilyId` mixups.
- **`Capability` match is exhaustive.** Adding a variant fails to compile until
  every role arm in `capabilities_of` is updated. Authorize against a
  `Capability` (`has`), not a `Role` directly.
- **Relationships are DB-shaped.** Callers must run `canonicalize_pair` before
  inserting a partnership (`CHECK partner_a_id < partner_b_id`) and
  `would_create_cycle` before a parent-link. The cycle check is in-memory;
  persistence still wraps check+insert in one SERIALIZABLE tx (TOCTOU).

## Adding a repo method (workflow)

1. Edit the **trait in `crates/domain/src/repos/foo.rs` FIRST** — add the
   `async fn` signature; extend `FooRepoError`/Row/Draft if needed.
2. Implement it in `crate-persistence` (SQLx); see `rust-foundations` for
   `sqlx-prepare` + `.sqlx` cache discipline.
3. Wire/use it from `crate-api` or `worker`.

A repo trait that compiles but isn't implemented is a `persistence` error, not
a `domain` one.

## How to test

Pure logic is unit-tested inline, no Docker:
`cargo test -p my-family-domain`.

## Common mistakes

| Symptom | Fix |
|---|---|
| Want to add a query in `domain` | No SQL here — only the trait signature. Impl goes in persistence. |
| `non-exhaustive patterns: Capability::X` | Add the new cap to every `Role` arm in `capabilities_of`. |
| Partnership insert hits the DB CHECK | Run `canonicalize_pair` first; store `(min, max)`. |
| Reaching for `Uuid` in a function arg | Use the matching `*Id` newtype. |
