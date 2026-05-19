use std::time::Duration;

use figment::Figment;
use figment::providers::Env;
use serde::Deserialize;
use thiserror::Error;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub app_env: AppEnv,
    pub log_format: LogFormat,
    pub rust_log: String,

    pub api_host: String,
    pub api_port: u16,
    pub api_public_url: String,
    pub web_public_url: String,
    pub cors_allowed_origins: String,
    pub api_enable_docs: bool,
    pub api_metrics_bind: String,

    pub database_url: String,
    pub database_max_connections: u32,
    pub database_acquire_timeout_seconds: u64,
    pub database_statement_timeout_ms: u32,

    pub redis_url: String,
    pub redis_max_connections: usize,
    pub redis_key_prefix: String,

    pub jwt_private_key: String,
    pub jwt_private_key_id: String,
    pub jwt_public_keys: String,
    pub jwt_issuer: String,
    pub jwt_audience: String,
    pub jwt_access_ttl_seconds: u64,
    pub jwt_refresh_ttl_seconds: u64,
    pub jwt_refresh_absolute_ttl_seconds: u64,

    pub cookie_domain: String,
    pub cookie_secure: bool,
    pub cookie_samesite_access: String,
    pub cookie_samesite_refresh: String,

    pub magic_link_ttl_seconds: u64,
    pub invite_ttl_seconds: u64,
    pub magic_link_rate_per_email_per_hour: u32,
    pub magic_link_rate_per_ip_per_hour: u32,

    pub email_dsn: String,
    pub email_from_name: String,
    pub email_from_address: String,
    pub email_reply_to: Option<String>,
    pub email_timeout_seconds: u64,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AppEnv {
    Development,
    Staging,
    Production,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum LogFormat {
    Pretty,
    Json,
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("figment: {0}")]
    Figment(#[from] Box<figment::Error>),
    #[error("validation: {0}")]
    Validation(String),
}

impl From<figment::Error> for ConfigError {
    fn from(err: figment::Error) -> Self {
        Self::Figment(Box::new(err))
    }
}

impl Config {
    pub fn load_from_env() -> Result<Self, ConfigError> {
        let cfg: Self = Figment::new().merge(Env::raw()).extract()?;
        cfg.validate()?;
        Ok(cfg)
    }

    pub const fn database_acquire_timeout(&self) -> Duration {
        Duration::from_secs(self.database_acquire_timeout_seconds)
    }

    fn validate(&self) -> Result<(), ConfigError> {
        if self.api_public_url.is_empty() {
            return Err(ConfigError::Validation("API_PUBLIC_URL required".into()));
        }
        if self.web_public_url.is_empty() {
            return Err(ConfigError::Validation("WEB_PUBLIC_URL required".into()));
        }
        if self.database_url.is_empty() {
            return Err(ConfigError::Validation("DATABASE_URL required".into()));
        }
        if self.redis_url.is_empty() {
            return Err(ConfigError::Validation("REDIS_URL required".into()));
        }
        if self.jwt_private_key.trim().is_empty() {
            return Err(ConfigError::Validation("JWT_PRIVATE_KEY required".into()));
        }
        if self.jwt_private_key_id.trim().is_empty() {
            return Err(ConfigError::Validation("JWT_PRIVATE_KEY_ID required".into()));
        }
        if self.jwt_public_keys.trim().is_empty() {
            return Err(ConfigError::Validation("JWT_PUBLIC_KEYS required".into()));
        }
        if !self.jwt_public_keys.contains(&self.jwt_private_key_id) {
            return Err(ConfigError::Validation(
                "JWT_PRIVATE_KEY_ID must appear as a kid in JWT_PUBLIC_KEYS".into(),
            ));
        }
        if self.email_dsn.is_empty() {
            return Err(ConfigError::Validation("EMAIL_DSN required".into()));
        }
        if self.email_from_address.is_empty() {
            return Err(ConfigError::Validation("EMAIL_FROM_ADDRESS required".into()));
        }
        if self.api_port == 0 {
            return Err(ConfigError::Validation("API_PORT must be non-zero".into()));
        }
        Ok(())
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    // `figment::Jail::expect_with` requires a closure returning `figment::Result<()>`,
    // whose `Err` variant is large by design; we cannot reshape it from here.
    clippy::result_large_err,
)]
mod tests {
    use figment::Jail;

    use super::*;

    const MINIMUM_ENV: &[(&str, &str)] = &[
        ("APP_ENV", "development"),
        ("LOG_FORMAT", "pretty"),
        ("RUST_LOG", "info"),
        ("API_HOST", "0.0.0.0"),
        ("API_PORT", "8080"),
        ("API_PUBLIC_URL", "http://localhost:8080"),
        ("WEB_PUBLIC_URL", "http://localhost:5173"),
        ("CORS_ALLOWED_ORIGINS", "http://localhost:5173"),
        ("API_ENABLE_DOCS", "true"),
        ("API_METRICS_BIND", "0.0.0.0:9090"),
        ("DATABASE_URL", "postgres://u:p@localhost/db"),
        ("DATABASE_MAX_CONNECTIONS", "10"),
        ("DATABASE_ACQUIRE_TIMEOUT_SECONDS", "5"),
        ("DATABASE_STATEMENT_TIMEOUT_MS", "30000"),
        ("REDIS_URL", "redis://localhost:6379/0"),
        ("REDIS_MAX_CONNECTIONS", "10"),
        ("REDIS_KEY_PREFIX", "my-family:"),
        ("JWT_PRIVATE_KEY", "dummy-pkcs8"),
        ("JWT_PRIVATE_KEY_ID", "dev-1"),
        ("JWT_PUBLIC_KEYS", "[{\"kid\":\"dev-1\",\"public_pem\":\"x\"}]"),
        ("JWT_ISSUER", "my-family"),
        ("JWT_AUDIENCE", "my-family-app"),
        ("JWT_ACCESS_TTL_SECONDS", "900"),
        ("JWT_REFRESH_TTL_SECONDS", "2592000"),
        ("JWT_REFRESH_ABSOLUTE_TTL_SECONDS", "7776000"),
        ("COOKIE_DOMAIN", ""),
        ("COOKIE_SECURE", "false"),
        ("COOKIE_SAMESITE_ACCESS", "Lax"),
        ("COOKIE_SAMESITE_REFRESH", "Strict"),
        ("MAGIC_LINK_TTL_SECONDS", "900"),
        ("INVITE_TTL_SECONDS", "1209600"),
        ("MAGIC_LINK_RATE_PER_EMAIL_PER_HOUR", "5"),
        ("MAGIC_LINK_RATE_PER_IP_PER_HOUR", "20"),
        ("EMAIL_DSN", "smtp://localhost:1025"),
        ("EMAIL_FROM_NAME", "my-family"),
        ("EMAIL_FROM_ADDRESS", "no-reply@my-family.local"),
        ("EMAIL_TIMEOUT_SECONDS", "10"),
    ];

    fn set_minimum_env(jail: &mut Jail) {
        for (k, v) in MINIMUM_ENV {
            jail.set_env(k, v);
        }
    }

    #[test]
    fn loads_full_config_from_env() {
        Jail::expect_with(|jail| {
            set_minimum_env(jail);
            let cfg = Config::load_from_env().expect("load");
            assert_eq!(cfg.app_env, AppEnv::Development);
            assert_eq!(cfg.api_port, 8080);
            assert_eq!(cfg.email_reply_to, None);
            Ok(())
        });
    }

    #[test]
    fn rejects_kid_mismatch() {
        Jail::expect_with(|jail| {
            set_minimum_env(jail);
            jail.set_env("JWT_PRIVATE_KEY_ID", "missing-kid");
            let err = Config::load_from_env().expect_err("should reject");
            assert!(matches!(err, ConfigError::Validation(_)));
            Ok(())
        });
    }
}
