---
name: crate-storage
description: Use when working with object storage in my-fam-tree — the my-fam-tree-storage crate that abstracts MinIO/S3 (prod) and a filesystem fallback (test/dev). Triggers — adding a new upload/download endpoint, wiring the store into AppState/WorkerState, deciding on storage key conventions, sanitising user-supplied keys, switching driver between S3 and Local. Keywords — ObjectStore trait, S3ObjectStore, LocalObjectStore, StorageError, S3Config, StorageConfig, StorageDriver, presigned_get, put, get, delete, AWS SDK, aws-sdk-s3, MinIO, path traversal, image upload, person photo, user avatar.
---

# crate-storage — S3-compatible object store

## Overview

`my-fam-tree-storage` is the standalone object-storage layer used by the api (and, when
async resize / cleanup jobs land, the worker). One `ObjectStore` trait, two impls:

- **`S3ObjectStore`** (prod, dev) — talks any S3-compatible endpoint via `aws-sdk-s3`.
  MinIO in dev (via the `minio-api` nginx proxy on port 80 — see `compose.yaml`), real
  AWS S3 in prod (leave `endpoint_url` unset, `force_path_style=false`).
- **`LocalObjectStore`** (tests, offline dev) — writes under a `base_dir`. `presigned_get`
  returns a `/api/v1/uploads/{key}`-style URL without any expiry enforcement.

Consumers store an `Arc<dyn ObjectStore>` (typically on `AppState::object_store`). The
trait is dyn-safe by design: no associated types, no generics in method signatures.

`src/lib.rs` re-exports: `ObjectStore`, `StorageError`, `S3ObjectStore`, `S3Config`,
`LocalObjectStore`, and `Bytes` (so consumers don't need a direct `bytes` dep).

## Module map

| Module    | Contents |
|-----------|----------|
| `store.rs`| `ObjectStore` trait (`put`, `get`, `delete`, `presigned_get`) + `StorageError` |
| `s3.rs`   | `S3Config`, `S3ObjectStore`, the AWS-SDK glue |
| `local.rs`| `LocalObjectStore`, tokio fs-backed impl |

## The `ObjectStore` trait

```rust
#[async_trait]
pub trait ObjectStore: Send + Sync + std::fmt::Debug {
    async fn put(&self, key: &str, content_type: &str, bytes: Bytes) -> Result<(), StorageError>;
    async fn get(&self, key: &str) -> Result<Bytes, StorageError>;
    async fn delete(&self, key: &str) -> Result<(), StorageError>;
    fn presigned_get(&self, key: &str, expires_in: Duration) -> Result<String, StorageError>;
}
```

`presigned_get` is intentionally synchronous; the S3 impl uses `block_in_place` +
`Handle::current()` so handler code stays straight-line. Don't make it `async` —
that ripples into every Person/User response serializer.

## Key conventions

Storage keys are flat strings (no leading slash, forward slashes for nesting):

| Concept           | Key shape                                              |
|-------------------|--------------------------------------------------------|
| Person photo      | `persons/{person_id}/{nanoid_or_uuid}.jpg`             |
| User avatar       | `users/{user_id}/{nanoid_or_uuid}.jpg`                 |

**Always include a random suffix** — uploading a new photo for the same person mints a
new key; the old one is deleted only after the DB update commits, so a request that
fails mid-flight never orphans the visible photo. Never recycle a key.

**Never let the user supply the key directly.** Callers ALWAYS construct the key from
trusted ids + a fresh random suffix. The LocalObjectStore is currently a `base_dir.join(key)`
— a key containing `..` would escape the base, which is why upstream sanitisation must
happen. Treat the trait as "trust me on the key" — the burden is on the caller.

## The two driver paths

The api binary builds the right impl from `cfg.storage` at startup:

```rust
let object_store: Arc<dyn ObjectStore> = match cfg.storage.driver {
    StorageDriver::S3 => Arc::new(S3ObjectStore::new(S3Config { … })),
    StorageDriver::Local => Arc::new(LocalObjectStore::new(
        PathBuf::from("target/uploads"),
        format!("{}/api/v1/uploads", cfg.api.public_url.trim_end_matches('/')),
    )),
};
```

Tests inject a per-test tempdir-backed `LocalObjectStore` (see
`crates/api/tests/common/mod.rs`). No tests touch real S3.

## Error model

`StorageError` is the only error type the trait surfaces:

```rust
pub enum StorageError {
    #[error("storage backend error: {0}")] Backend(String),
    #[error("object not found: {0}")]      NotFound(String),
    #[error("invalid storage configuration: {0}")] Config(String),
}
```

The S3 impl distinguishes `NoSuchKey` (→ `NotFound`) from every other AWS service
error (→ `Backend`); LocalObjectStore mirrors that with `ErrorKind::NotFound`.
Handlers map these to api-level `ApiError` (`NotFound` → 404, `Backend` → 500). Don't
leak the inner string verbatim into client responses — it can contain bucket names
and request IDs.

## When to extend the trait

- **New op (e.g. `head` to probe existence without download)** — add to the trait,
  implement in both backends, update the doc comment to spell out the contract
  (especially around the NotFound vs Backend distinction).
- **New driver (e.g. Cloudflare R2 with custom auth)** — add a new file under
  `src/`, re-export from `lib.rs`, add a `StorageDriver` variant in `crate-config`,
  thread it through `bin/api.rs`. The trait should stay agnostic.

## What lives elsewhere

- **Validation + resize** — `crates/api/src/images.rs`. Magic-byte detection,
  extension cross-check, raw-size cap, JPEG re-encode at quality 80, max
  dimension 512 px. This is api-policy, not storage. Don't push it into this crate.
- **Storage config** — `crates/config/src/storage.rs` defines `StorageDriver`,
  `StorageConfig`, the env-var names. See `crate-config`.
- **MinIO compose setup** — `compose.yaml` runs two services: `minio` (S3 API on
  9000 + console on 9001, exposed on `http://minio.my-fam-tree.docker`), and
  `minio-api` (nginx-unprivileged reverse-proxying the S3 API on port 80 via
  `http://minio-api.my-fam-tree.docker`). The api/worker hit the proxy hostname so
  presigned URLs don't leak `:9000`.

## Latent footgun to remember

`LocalObjectStore::path_for(key)` does a blind `base_dir.join(key)`. A `..` in the
key escapes. We currently rely on the caller validating, but the audit flagged this
as latent — when the photo/avatar endpoints land, the route handler must construct
the key from server-side ids only (never echo a client-supplied key into `put`).
Add an in-store sanity check later: reject keys containing `..`, leading `/`, or NUL.
