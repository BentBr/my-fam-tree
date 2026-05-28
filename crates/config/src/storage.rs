//! Object-storage configuration. Drives `crates/storage` for both the api
//! (uploads) and the worker (cleanup / future async-resize jobs).

use serde::Deserialize;

/// Which backend the storage crate should build.
///
/// `s3` covers real AWS, `MinIO`, Cloudflare `R2`; `local` writes to the
/// host filesystem (tests + offline dev that doesn't want `MinIO` running).
#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum StorageDriver {
    S3,
    Local,
}

/// Everything the storage crate needs to build either backend.
///
/// Some fields are `S3`-specific (region, keys, endpoint, force-path-style);
/// the `Local` driver ignores them but they stay populated so an operator
/// can flip `STORAGE_DRIVER` without re-editing the rest of the env.
#[derive(Debug, Clone)]
pub struct StorageConfig {
    pub driver: StorageDriver,
    pub bucket: String,
    pub region: String,
    pub endpoint_url: Option<String>,
    pub access_key_id: String,
    pub secret_access_key: String,
    pub force_path_style: bool,
}
