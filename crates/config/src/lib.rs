//! Centralised configuration for every binary in the workspace.
//!
//! Each binary (api / worker / seeder / migrator) loads ONE struct from the
//! process environment via [`ApiConfig::from_env`] / [`WorkerConfig::from_env`].
//! Common building blocks ([`DatabaseConfig`], [`RedisConfig`], [`EmailConfig`],
//! [`StorageConfig`], …) are typed sub-structs reused across the per-binary
//! configs, so when a knob lands once here every binary picks it up.
//!
//! Loading uses [`figment`] with the `Env::raw()` provider — env names map
//! 1-to-1 to the flat names you'd write in `.env` (`DATABASE_URL`,
//! `JWT_PRIVATE_KEY`, …). The nested public surface (`cfg.database.url`,
//! `cfg.jwt.private_key`) is built from a private flat intermediate so we
//! get serde's whole-struct validation without forcing operators to write
//! double-underscore env vars.

#![allow(
    clippy::struct_field_names,
    clippy::struct_excessive_bools,
    reason = "config structs mirror env var names; bools are flags by nature"
)]

use figment::Figment;
use figment::providers::Env;
use serde::Deserialize;

pub mod api;
pub mod common;
pub mod storage;
pub mod worker;

pub use api::{ApiBindConfig, ApiConfig, CookieConfig, JwtConfig, MagicLinkConfig};
pub use common::{
    AppEnv, DatabaseConfig, EmailConfig, LogConfig, LogFormat, RedisConfig, WebConfig,
};
pub use storage::StorageConfig;
pub use worker::{JanitorConfig, OutboxConfig, WorkerConfig, WorkerLoopConfig};

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("missing or invalid env var: {0}")]
    Env(String),
    #[error("validation: {0}")]
    Validation(String),
}

impl From<figment::Error> for ConfigError {
    fn from(e: figment::Error) -> Self {
        Self::Env(e.to_string())
    }
}

/// Deserialize a flat env-shaped intermediate via figment + the process env.
/// Each binary's `from_env()` calls this with its private flat struct then
/// shapes the public nested config from the result.
pub(crate) fn load_flat<T: for<'de> Deserialize<'de>>() -> Result<T, ConfigError> {
    Ok(Figment::new().merge(Env::raw()).extract()?)
}
