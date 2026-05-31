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
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Development => "development",
            Self::Staging => "staging",
            Self::Production => "production",
        }
    }
}

impl std::fmt::Display for AppEnv {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_env_is_development_only_for_development() {
        assert!(AppEnv::Development.is_development());
        assert!(!AppEnv::Staging.is_development());
        assert!(!AppEnv::Production.is_development());
    }

    #[test]
    fn app_env_as_str_matches_serde_lowercase_form() {
        assert_eq!(AppEnv::Development.as_str(), "development");
        assert_eq!(AppEnv::Staging.as_str(), "staging");
        assert_eq!(AppEnv::Production.as_str(), "production");
        assert_eq!(format!("{}", AppEnv::Production), "production");
    }

    // AppEnv + LogFormat deserialisation under serde's
    // `rename_all = "lowercase"` is exercised end-to-end by the worker /
    // api `from_env` Jail tests — they pump real env-string values
    // through figment::Env and assert the resulting `AppEnv` / `LogFormat`.
    // Replicating that with serde_json here would need a dep this crate
    // doesn't carry, so the pure-type assertions above are sufficient.
}
