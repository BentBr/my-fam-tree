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
// NB: NO `#[serde(deny_unknown_fields)]` — figment's `Env::raw()` merges the
// whole process environment (PATH, HOME, the shell's `_`, …) and the
// deserializer would reject every unknown env var the system happens to have.
// We rely on REQUIRED fields being absent to surface configuration mistakes.
#[derive(Deserialize)]
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
    /// Load the api configuration from the process environment and run
    /// cross-field validation (`JWT_PRIVATE_KEY_ID` must appear as a kid
    /// in `JWT_PUBLIC_KEYS`, critical strings are non-empty, …).
    ///
    /// # Errors
    /// Returns [`ConfigError::Env`] if any required env var is missing or
    /// fails to parse, or [`ConfigError::Validation`] if a cross-field
    /// invariant fails.
    pub fn from_env() -> Result<Self, ConfigError> {
        let f: FlatApiConfig = load_flat()?;
        let cfg = Self::build(f);
        cfg.validate()?;
        Ok(cfg)
    }

    fn validate(&self) -> Result<(), ConfigError> {
        if self.api.public_url.is_empty() {
            return Err(ConfigError::Validation("API_PUBLIC_URL required".into()));
        }
        if self.web.public_url.is_empty() {
            return Err(ConfigError::Validation("WEB_PUBLIC_URL required".into()));
        }
        if self.database.url.is_empty() {
            return Err(ConfigError::Validation("DATABASE_URL required".into()));
        }
        if self.redis.url.is_empty() {
            return Err(ConfigError::Validation("REDIS_URL required".into()));
        }
        if self.jwt.private_key.trim().is_empty() {
            return Err(ConfigError::Validation("JWT_PRIVATE_KEY required".into()));
        }
        if self.jwt.private_key_id.trim().is_empty() {
            return Err(ConfigError::Validation("JWT_PRIVATE_KEY_ID required".into()));
        }
        if self.jwt.public_keys.trim().is_empty() {
            return Err(ConfigError::Validation("JWT_PUBLIC_KEYS required".into()));
        }
        if !self.jwt.public_keys.contains(&self.jwt.private_key_id) {
            return Err(ConfigError::Validation(
                "JWT_PRIVATE_KEY_ID must appear as a kid in JWT_PUBLIC_KEYS".into(),
            ));
        }
        if self.email.dsn.is_empty() {
            return Err(ConfigError::Validation("EMAIL_DSN required".into()));
        }
        if self.email.from_address.is_empty() {
            return Err(ConfigError::Validation("EMAIL_FROM_ADDRESS required".into()));
        }
        if self.api.port == 0 {
            return Err(ConfigError::Validation("API_PORT must be non-zero".into()));
        }
        // Security audit LOW. An empty list silently registers a single
        // empty-string origin with actix-cors; a literal "*" combined with
        // `supports_credentials()` is forbidden by the CORS spec and
        // behaviour depends on the actix-cors version. Catch both at boot.
        let allowed: Vec<&str> = self
            .api
            .cors_allowed_origins
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .collect();
        if allowed.is_empty() {
            return Err(ConfigError::Validation(
                "CORS_ALLOWED_ORIGINS must list at least one origin (comma-separated)".into(),
            ));
        }
        for origin in &allowed {
            if *origin == "*" {
                return Err(ConfigError::Validation(
                    "CORS_ALLOWED_ORIGINS must not contain `*` while credentials are supported"
                        .into(),
                ));
            }
            // Each entry must parse as an http(s) URL — guards against
            // typos like "https//example.com" (missing colon) which
            // actix-cors would silently accept as a bogus origin.
            let parsed = url::Url::parse(origin).map_err(|e| {
                ConfigError::Validation(format!(
                    "CORS_ALLOWED_ORIGINS entry `{origin}` is not a valid URL: {e}",
                ))
            })?;
            if !matches!(parsed.scheme(), "http" | "https") {
                return Err(ConfigError::Validation(format!(
                    "CORS_ALLOWED_ORIGINS entry `{origin}` must use http or https",
                )));
            }
        }
        // Security audit MEDIUM. In production we MUST ship cookies marked
        // Secure (so browsers refuse them over plain HTTP) and the refresh
        // cookie MUST be SameSite=Strict — without these, a misconfigured
        // prod deploy hands every session cookie to the network.
        if matches!(self.app_env, AppEnv::Production) {
            if !self.cookie.secure {
                return Err(ConfigError::Validation(
                    "COOKIE_SECURE must be true in production".into(),
                ));
            }
            if self.cookie.samesite_refresh != "Strict" {
                return Err(ConfigError::Validation(
                    "COOKIE_SAMESITE_REFRESH must be `Strict` in production".into(),
                ));
            }
        }
        Ok(())
    }

    fn build(f: FlatApiConfig) -> Self {
        Self {
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
        }
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::result_large_err,
    reason = "figment::Jail::expect_with's closure returns figment::Result; the large-err variant is its design"
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
        ("API_ENABLE_DOCS", "false"),
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
        ("STORAGE_DRIVER", "local"),
        ("STORAGE_BUCKET", "my-family"),
        ("STORAGE_REGION", "us-east-1"),
        ("STORAGE_ACCESS_KEY_ID", ""),
        ("STORAGE_SECRET_ACCESS_KEY", ""),
        ("STORAGE_FORCE_PATH_STYLE", "false"),
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
            let cfg = ApiConfig::from_env().expect("load");
            assert_eq!(cfg.app_env, AppEnv::Development);
            assert_eq!(cfg.api.port, 8080);
            assert_eq!(cfg.email.reply_to, None);
            assert_eq!(cfg.database.url, "postgres://u:p@localhost/db");
            assert_eq!(cfg.storage.bucket, "my-family");
            Ok(())
        });
    }

    #[test]
    fn rejects_kid_mismatch() {
        Jail::expect_with(|jail| {
            set_minimum_env(jail);
            jail.set_env("JWT_PRIVATE_KEY_ID", "missing-kid");
            let err = ApiConfig::from_env().expect_err("should reject");
            assert!(matches!(err, ConfigError::Validation(_)));
            Ok(())
        });
    }

    #[test]
    fn prod_rejects_insecure_cookie() {
        Jail::expect_with(|jail| {
            set_minimum_env(jail);
            jail.set_env("APP_ENV", "production");
            // COOKIE_SECURE defaults to false in the minimum env — that's
            // fine for development but must fail validate() in production.
            let err = ApiConfig::from_env().expect_err("prod with secure=false must reject");
            let ConfigError::Validation(msg) = err else {
                unreachable!("expected Validation; got {err:?}");
            };
            assert!(msg.contains("COOKIE_SECURE"));
            Ok(())
        });
    }

    #[test]
    fn prod_rejects_lax_refresh_samesite() {
        Jail::expect_with(|jail| {
            set_minimum_env(jail);
            jail.set_env("APP_ENV", "production");
            jail.set_env("COOKIE_SECURE", "true");
            jail.set_env("COOKIE_SAMESITE_REFRESH", "Lax");
            let err = ApiConfig::from_env().expect_err("prod with lax refresh must reject");
            let ConfigError::Validation(msg) = err else {
                unreachable!("expected Validation; got {err:?}");
            };
            assert!(msg.contains("COOKIE_SAMESITE_REFRESH"));
            Ok(())
        });
    }

    #[test]
    fn prod_accepts_secure_and_strict_refresh() {
        Jail::expect_with(|jail| {
            set_minimum_env(jail);
            jail.set_env("APP_ENV", "production");
            jail.set_env("COOKIE_SECURE", "true");
            // COOKIE_SAMESITE_REFRESH defaults to Strict in the minimum env.
            ApiConfig::from_env().expect("prod with secure+strict must load");
            Ok(())
        });
    }

    #[test]
    fn rejects_empty_cors_origins() {
        Jail::expect_with(|jail| {
            set_minimum_env(jail);
            jail.set_env("CORS_ALLOWED_ORIGINS", "");
            let err = ApiConfig::from_env().expect_err("empty CORS list must reject");
            let ConfigError::Validation(msg) = err else {
                unreachable!("expected Validation; got {err:?}");
            };
            assert!(msg.contains("CORS_ALLOWED_ORIGINS"));
            Ok(())
        });
    }

    #[test]
    fn rejects_wildcard_cors_origin() {
        Jail::expect_with(|jail| {
            set_minimum_env(jail);
            jail.set_env("CORS_ALLOWED_ORIGINS", "*");
            let err = ApiConfig::from_env().expect_err("`*` with credentials must reject");
            let ConfigError::Validation(msg) = err else {
                unreachable!("expected Validation; got {err:?}");
            };
            assert!(msg.contains('*'));
            Ok(())
        });
    }

    #[test]
    fn rejects_non_url_cors_origin() {
        Jail::expect_with(|jail| {
            set_minimum_env(jail);
            // Missing colon — looks like a URL but doesn't parse.
            jail.set_env("CORS_ALLOWED_ORIGINS", "https//example.com");
            let err = ApiConfig::from_env().expect_err("bad URL must reject");
            let ConfigError::Validation(msg) = err else {
                unreachable!("expected Validation; got {err:?}");
            };
            assert!(msg.contains("not a valid URL") || msg.contains("must use http"));
            Ok(())
        });
    }

    #[test]
    fn rejects_non_http_cors_origin() {
        Jail::expect_with(|jail| {
            set_minimum_env(jail);
            // Valid URL shape, wrong scheme.
            jail.set_env("CORS_ALLOWED_ORIGINS", "file:///etc/passwd");
            let err = ApiConfig::from_env().expect_err("non-http scheme must reject");
            let ConfigError::Validation(msg) = err else {
                unreachable!("expected Validation; got {err:?}");
            };
            assert!(msg.contains("http"));
            Ok(())
        });
    }
}
