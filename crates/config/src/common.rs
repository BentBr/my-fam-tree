//! Sub-configs shared by every binary in the workspace.

use serde::Deserialize;

/// Deployment environment. Drives the dotenv autoload + a couple of
/// safety toggles (e.g. cookie `Secure`).
#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AppEnv {
    Development,
    Staging,
    Production,
}

impl AppEnv {
    #[must_use]
    pub const fn is_development(self) -> bool {
        matches!(self, Self::Development)
    }
}

/// Log output format. `pretty` for dev, `json` for prod / ELK ingestion.
#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum LogFormat {
    Pretty,
    Json,
}

/// Tracing-subscriber configuration. `level` is the env-filter string
/// (`RUST_LOG`); `format` toggles the layer.
#[derive(Debug, Clone)]
pub struct LogConfig {
    pub level: String,
    pub format: LogFormat,
}

/// Postgres connection pool tuning. Same shape for every binary.
#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
    pub acquire_timeout_seconds: u64,
    pub statement_timeout_ms: u32,
}

/// Redis connection pool tuning + the global key namespace.
#[derive(Debug, Clone)]
pub struct RedisConfig {
    pub url: String,
    pub max_connections: usize,
    pub key_prefix: String,
}

/// Outbound mail transport + envelope defaults. The worker actually
/// sends; the api just enqueues into the outbox using these defaults.
#[derive(Debug, Clone)]
pub struct EmailConfig {
    pub dsn: String,
    pub from_name: String,
    pub from_address: String,
    pub reply_to: Option<String>,
    pub timeout_seconds: u64,
}

/// Public-facing web URL — used as the link base in outbound emails so a
/// magic-link / invite URL points at the right host.
#[derive(Debug, Clone)]
pub struct WebConfig {
    pub public_url: String,
}
