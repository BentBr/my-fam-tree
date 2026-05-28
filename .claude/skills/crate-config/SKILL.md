---
name: crate-config
description: Use when reading or changing how my-family loads runtime configuration — the centralised my-family-config crate that backs ApiConfig and WorkerConfig. Triggers — adding a new env var; renaming/removing one; tweaking validation (JWT kid, production cookie flags, storage driver); wiring a new field into AppState or WorkerState; debugging "missing/invalid env var" errors. Keywords — figment, Env::raw, ApiConfig, WorkerConfig, FlatApiConfig, FlatWorkerConfig, load_flat, ConfigError, AppEnv, LogFormat, StorageDriver, from_env, validate.
---

# crate-config — single source of truth for env-derived config

## Overview

`my-family-config` is the ONLY crate that reads the process environment. Every other crate
takes a fully-typed `ApiConfig` / `WorkerConfig` (or sub-config) by reference; raw `std::env`
lookups outside this crate are a bug. The split mirrors the binaries: `ApiConfig` powers the
api and the seeder, `WorkerConfig` powers the worker. Shared sub-configs live in `common.rs`
/ `storage.rs` so a future binary can compose its own top-level type without duplicating
field shapes.

`src/lib.rs` re-exports the public surface: `ApiConfig`, `WorkerConfig`, `ConfigError`,
`AppEnv`, `LogFormat`, plus every sub-config (`DatabaseConfig`, `RedisConfig`,
`EmailConfig`, `WebConfig`, `JwtConfig`, `CookieConfig`, `MagicLinkConfig`,
`ApiBindConfig`, `StorageConfig`, `WorkerLoopConfig`, `JanitorConfig`, `OutboxConfig`,
`StorageDriver`).

## Module map

| Module | Contents |
|---|---|
| `lib.rs`     | `ConfigError`, `load_flat<T>()`, public re-exports |
| `common.rs`  | `AppEnv` (Development/Production/Test + `is_development` + `Display`), `LogFormat`, `LogConfig`, `DatabaseConfig`, `RedisConfig`, `EmailConfig`, `WebConfig` |
| `storage.rs` | `StorageDriver` enum (S3/Local), `StorageConfig` |
| `api.rs`     | `ApiConfig` + private `FlatApiConfig` + `ApiBindConfig`/`JwtConfig`/`CookieConfig`/`MagicLinkConfig` + `from_env` + `validate` |
| `worker.rs`  | `WorkerConfig` + private `FlatWorkerConfig` + `WorkerLoopConfig`/`JanitorConfig`/`OutboxConfig` + `from_env` |

## The two-struct pattern

Every top-level config follows a strict **Flat → Nested → Validate** flow:

1. **Private flat struct (`FlatApiConfig` / `FlatWorkerConfig`)** — one field per env var,
   names matching the env name verbatim in snake_case (`database_url`, `jwt_private_key`,
   `storage_endpoint_url`). `#[derive(Deserialize)]` only — **NEVER add
   `#[serde(deny_unknown_fields)]`**. figment's `Env::raw()` provider merges the entire
   process environment (PATH, HOME, the shell's `_`, …) and serde would reject every
   unknown var the system happens to have. Required fields surface mistakes via the
   `missing field` path.
2. **Public nested struct (`ApiConfig` / `WorkerConfig`)** — grouped by concern
   (`cfg.database.url`, `cfg.jwt.private_key`). Hand-written `From<FlatX>` constructs it,
   so callers don't see the flat shape.
3. **`validate()`** — runs after construction. Catches cross-field invariants the type
   system can't (JWT `kid` matches a key in `public_keys`, `COOKIE_SECURE=true` in
   production, non-empty critical strings, samesite spellings, `cors_allowed_origins`
   shape).

`from_env()` is the only public entry point and chains all three:

```rust
pub fn from_env() -> Result<Self, ConfigError> {
    let f: FlatApiConfig = load_flat()?;   // figment + Env::raw + serde
    let cfg: Self = f.into();              // private Flat → public Nested
    cfg.validate()?;                       // cross-field invariants
    Ok(cfg)
}
```

`load_flat<T>()` is `crate::load_flat` — the single figment call site:

```rust
Figment::new().merge(Env::raw()).extract::<T>()
```

If you add a new top-level config (e.g. a future `JobsConfig`), copy this exact shape
— do not invent a second loader, do not expose figment to consumers.

## Adding an env var (recipe)

1. **Decide the group.** Reuse an existing sub-config if it fits (most do). Only add
   a new sub-config when the field doesn't belong to any current group.
2. **Add the field to the public sub-config** (`pub host: String`, `pub timeout_seconds: u64`,
   …). Match naming: snake_case, ASCII, no `Option` unless absence is genuinely meaningful
   (most "I forgot to set it" cases want a required field that surfaces at startup).
3. **Add the matching field to the private `FlatApiConfig` / `FlatWorkerConfig`**, named
   exactly as the lower-cased env var (`api_metrics_bind`, `magic_link_ttl_seconds`).
4. **Populate it in the `From<FlatX> for Foo`** impl.
5. **Document + default** in `.env.example` AND `compose.yaml` (`x-rust-env` anchor).
   Both must list the variable; CI's `.github/workflows/ci.yml` `env:` blocks must too
   (`backend-tests`, `backend-coverage`, `frontend-e2e`).
6. **Validate** anything non-trivial in `validate()` — empty-string check for required
   strings, parseable URL, etc.

## Consuming config — conventions

- **Pass `&Config` (or a sub-config) by reference.** Storing `Arc<Config>` only makes sense
  inside `AppState` / `WorkerState`. Handler code reads `state.cfg.jwt.access_ttl_seconds`,
  not a cloned struct.
- **No `std::env::var(...)` outside this crate.** If a piece of code needs a value, it
  goes through `Config`. The single exception is dotenvy in the binary entry-points —
  load `.env` BEFORE `Config::from_env()`.
- **Test injection.** Integration tests build a `Config` literal (see
  `crates/api/tests/common/mod.rs::test_cfg()`) — no env round-trip. Use the same
  shape; if a sub-config changes, update the helper.
- **`figment::Jail` in unit tests.** Inside `crates/config/src/api.rs` the unit tests
  scope env mutations through `figment::Jail::expect_with` (behind the
  `figment/test` dev-dep feature flag). Mirror that pattern when adding load/validate
  tests; never mutate `std::env` directly from a test.

## Error model

`ConfigError` is a public, transparent two-variant enum:

```rust
pub enum ConfigError {
    #[error("missing or invalid env var: {0}")]
    Env(String),       // wraps a figment::Error
    #[error("invalid configuration: {0}")]
    Validation(String) // cross-field invariant failures
}
```

Both variants surface to the binary entry-point via `anyhow::Context` (e.g.
`Config::from_env().context("load config from environment")?`). Binaries print the chain
and exit non-zero — no recovery, no silent defaults. Don't add an `Other` variant; if
you need a new error class, add a named variant with a precise message.

## Conventions to defend

- **One config crate** — there is no `api/src/config.rs` or `worker/src/config.rs`. If
  a PR re-introduces a per-binary config struct, push back and migrate it into this crate.
- **No string-typed log/env enums leaking through.** `AppEnv` and `LogFormat` are real
  enums; consumers `matches!(cfg.app_env, AppEnv::Production)`, never `cfg.app_env ==
  "production"`.
- **`validate()` runs before the binary touches the network.** Adding a new
  sanity-check is cheap and free at runtime (one-time at startup). Add the check.

## When the load path itself misbehaves

Symptom: `unknown field: found \`home\`, expected one of …`. Cause: someone re-added
`#[serde(deny_unknown_fields)]` to a Flat struct. Remove it; see the file-level NB
comment for the rationale.

Symptom: binary loads fine in CI but breaks locally with `missing field` X. Cause: a
required env var landed on main without an update to your `.env`. Diff
`.env.example` to find what to add.

Symptom: tests pass on host but `cargo run -p my-family-seeder` fails inside the
network container. Cause: `scripts/cargo-in-network.sh` only forwards a small set
of env vars (DB/Redis/Mailpit). Pass others through `docker run --env-file .env`,
or extend the script if it's a permanent need.
