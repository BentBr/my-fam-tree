//! `ApiConfig` — everything the `api` binary reads from the environment.

use serde::Deserialize;

use crate::common::{
    AppEnv, DatabaseConfig, EmailConfig, LogConfig, LogFormat, RedisConfig, WebConfig,
};
use crate::storage::{StorageConfig, StorageDriver};
use crate::{ConfigError, load_flat};

/// Signed-token issuance + verification. The private key signs new
/// access/refresh JWTs; `public_keys` is a JSON-encoded JWKS that the
/// middleware checks against (rotation-friendly).
#[derive(Debug, Clone)]
pub struct JwtConfig {
    pub private_key: String,
    pub private_key_id: String,
    pub public_keys: String,
    pub issuer: String,
    pub audience: String,
    pub access_ttl_seconds: u64,
    pub refresh_ttl_seconds: u64,
    pub refresh_absolute_ttl_seconds: u64,
}

/// Browser cookie attributes. `secure` should be true in prod; the
/// `SameSite` policy differs for access vs refresh because the refresh
/// token only ever flows to the dedicated refresh endpoint.
#[derive(Debug, Clone)]
pub struct CookieConfig {
    pub domain: String,
    pub secure: bool,
    pub samesite_access: String,
    pub samesite_refresh: String,
}

/// Magic-link + invite token rules. All TTLs are seconds.
#[derive(Debug, Clone)]
pub struct MagicLinkConfig {
    pub ttl_seconds: u64,
    pub invite_ttl_seconds: u64,
    pub rate_per_email_per_hour: u32,
    pub rate_per_ip_per_hour: u32,
}

/// HTTP server bind + cross-origin + docs toggles.
#[derive(Debug, Clone)]
pub struct ApiBindConfig {
    pub host: String,
    pub port: u16,
    pub public_url: String,
    pub cors_allowed_origins: String,
    pub enable_docs: bool,
    pub metrics_bind: String,
}

/// What the `api` binary loads from the environment.
#[derive(Debug, Clone)]
pub struct ApiConfig {
    pub app_env: AppEnv,
    pub log: LogConfig,
    pub database: DatabaseConfig,
    pub redis: RedisConfig,
    pub email: EmailConfig,
    pub web: WebConfig,
    pub storage: StorageConfig,
    pub api: ApiBindConfig,
    pub jwt: JwtConfig,
    pub cookie: CookieConfig,
    pub magic_link: MagicLinkConfig,
}

/// Private flat shape that mirrors env-var names verbatim, letting figment
/// and serde populate it without any double-underscore dance. The public
/// `ApiConfig` is built from this in `from_env`.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FlatApiConfig {
    app_env: AppEnv,
    rust_log: String,
    log_format: LogFormat,

    database_url: String,
    database_max_connections: u32,
    database_acquire_timeout_seconds: u64,
    database_statement_timeout_ms: u32,

    redis_url: String,
    redis_max_connections: usize,
    redis_key_prefix: String,

    email_dsn: String,
    email_from_name: String,
    email_from_address: String,
    email_reply_to: Option<String>,
    email_timeout_seconds: u64,

    web_public_url: String,

    storage_driver: StorageDriver,
    storage_bucket: String,
    storage_region: String,
    storage_endpoint_url: Option<String>,
    storage_access_key_id: String,
    storage_secret_access_key: String,
    storage_force_path_style: bool,

    api_host: String,
    api_port: u16,
    api_public_url: String,
    cors_allowed_origins: String,
    api_enable_docs: bool,
    api_metrics_bind: String,

    jwt_private_key: String,
    jwt_private_key_id: String,
    jwt_public_keys: String,
    jwt_issuer: String,
    jwt_audience: String,
    jwt_access_ttl_seconds: u64,
    jwt_refresh_ttl_seconds: u64,
    jwt_refresh_absolute_ttl_seconds: u64,

    cookie_domain: String,
    cookie_secure: bool,
    cookie_samesite_access: String,
    cookie_samesite_refresh: String,

    magic_link_ttl_seconds: u64,
    invite_ttl_seconds: u64,
    magic_link_rate_per_email_per_hour: u32,
    magic_link_rate_per_ip_per_hour: u32,
}

impl ApiConfig {
    /// Load the api configuration from the process environment.
    ///
    /// # Errors
    /// Returns [`ConfigError`] if any required env var is missing or
    /// fails to parse into its typed shape.
    pub fn from_env() -> Result<Self, ConfigError> {
        let f: FlatApiConfig = load_flat()?;
        Ok(Self {
            app_env: f.app_env,
            log: LogConfig { level: f.rust_log, format: f.log_format },
            database: DatabaseConfig {
                url: f.database_url,
                max_connections: f.database_max_connections,
                acquire_timeout_seconds: f.database_acquire_timeout_seconds,
                statement_timeout_ms: f.database_statement_timeout_ms,
            },
            redis: RedisConfig {
                url: f.redis_url,
                max_connections: f.redis_max_connections,
                key_prefix: f.redis_key_prefix,
            },
            email: EmailConfig {
                dsn: f.email_dsn,
                from_name: f.email_from_name,
                from_address: f.email_from_address,
                reply_to: f.email_reply_to,
                timeout_seconds: f.email_timeout_seconds,
            },
            web: WebConfig { public_url: f.web_public_url },
            storage: StorageConfig {
                driver: f.storage_driver,
                bucket: f.storage_bucket,
                region: f.storage_region,
                endpoint_url: f.storage_endpoint_url,
                access_key_id: f.storage_access_key_id,
                secret_access_key: f.storage_secret_access_key,
                force_path_style: f.storage_force_path_style,
            },
            api: ApiBindConfig {
                host: f.api_host,
                port: f.api_port,
                public_url: f.api_public_url,
                cors_allowed_origins: f.cors_allowed_origins,
                enable_docs: f.api_enable_docs,
                metrics_bind: f.api_metrics_bind,
            },
            jwt: JwtConfig {
                private_key: f.jwt_private_key,
                private_key_id: f.jwt_private_key_id,
                public_keys: f.jwt_public_keys,
                issuer: f.jwt_issuer,
                audience: f.jwt_audience,
                access_ttl_seconds: f.jwt_access_ttl_seconds,
                refresh_ttl_seconds: f.jwt_refresh_ttl_seconds,
                refresh_absolute_ttl_seconds: f.jwt_refresh_absolute_ttl_seconds,
            },
            cookie: CookieConfig {
                domain: f.cookie_domain,
                secure: f.cookie_secure,
                samesite_access: f.cookie_samesite_access,
                samesite_refresh: f.cookie_samesite_refresh,
            },
            magic_link: MagicLinkConfig {
                ttl_seconds: f.magic_link_ttl_seconds,
                invite_ttl_seconds: f.invite_ttl_seconds,
                rate_per_email_per_hour: f.magic_link_rate_per_email_per_hour,
                rate_per_ip_per_hour: f.magic_link_rate_per_ip_per_hour,
            },
        })
    }
}
