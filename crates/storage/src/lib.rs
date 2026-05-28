//! Object-store abstraction for binary blobs (person photos at v1).
//!
//! - The [`ObjectStore`] trait describes the operations the `api` + `worker`
//!   need: `put`, `get`, `delete`, and `presigned_get` (a URL the FE can
//!   `<img src>` directly, without proxying bytes through the `api`).
//! - [`S3ObjectStore`] talks to any `S3`-compatible endpoint — real AWS
//!   `S3`, `MinIO` in dev / on-prem, Cloudflare `R2`. `MinIO` needs
//!   `force_path_style = true` and a custom endpoint URL; production
//!   `S3` uses neither.
//! - [`LocalObjectStore`] writes to the host filesystem under a configured
//!   base directory + returns "presigned" URLs that point at a local route.
//!   Used in `cargo test` so nothing needs `MinIO` running.

mod error;
mod local;
mod s3;
mod store;

pub use bytes::Bytes;
pub use error::StorageError;
pub use local::LocalObjectStore;
pub use s3::{S3Config, S3ObjectStore};
pub use store::ObjectStore;
