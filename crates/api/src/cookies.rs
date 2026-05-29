//! Typed cookie builders for access + refresh tokens.
//!
//! Centralised so the `HttpOnly`, `Secure`, `SameSite`, `Path`, and `Max-Age`
//! flags can never drift between the issuing handlers (`/auth/consume`,
//! `/auth/refresh`, `/families`, `/invites/accept`) and the revoking handler
//! (`/auth/logout`).
//!
//! The refresh cookie's `Path` is scoped to `/api/v1/auth/refresh` so it is
//! only sent on the single endpoint that needs it — narrowing the blast radius
//! if any other endpoint is ever compromised.

use actix_web::cookie::time::Duration;
use actix_web::cookie::{Cookie, SameSite};

use crate::Config;

pub const ACCESS_COOKIE: &str = "access";
pub const REFRESH_COOKIE: &str = "refresh";
pub const REFRESH_COOKIE_PATH: &str = "/api/v1/auth/refresh";

fn parse_samesite(s: &str) -> SameSite {
    match s {
        "Strict" => SameSite::Strict,
        "None" => SameSite::None,
        _ => SameSite::Lax,
    }
}

fn seconds(value: u64) -> Duration {
    Duration::seconds(i64::try_from(value).unwrap_or(i64::MAX))
}

/// Build the short-lived access cookie. Sent on every API request via path `/`.
#[must_use]
pub fn access_cookie<'a>(cfg: &Config, value: String) -> Cookie<'a> {
    let mut c = Cookie::build(ACCESS_COOKIE, value)
        .http_only(true)
        .secure(cfg.cookie.secure)
        .same_site(parse_samesite(&cfg.cookie.samesite_access))
        .path("/")
        .max_age(seconds(cfg.jwt.access_ttl_seconds))
        .finish();
    if !cfg.cookie.domain.is_empty() {
        c.set_domain(cfg.cookie.domain.clone());
    }
    c
}

/// Build the long-lived refresh cookie. Scoped to the `/auth/refresh` path so
/// the browser will never attach it to other endpoints.
#[must_use]
pub fn refresh_cookie<'a>(cfg: &Config, value: String) -> Cookie<'a> {
    let mut c = Cookie::build(REFRESH_COOKIE, value)
        .http_only(true)
        .secure(cfg.cookie.secure)
        .same_site(parse_samesite(&cfg.cookie.samesite_refresh))
        .path(REFRESH_COOKIE_PATH)
        .max_age(seconds(cfg.jwt.refresh_ttl_seconds))
        .finish();
    if !cfg.cookie.domain.is_empty() {
        c.set_domain(cfg.cookie.domain.clone());
    }
    c
}

/// Build a zero-value, zero-age cookie used by `/auth/logout`.
///
/// The domain must match the one used when issuing — without it, browsers
/// treat the `Set-Cookie` as a different scope and the original
/// (domain-scoped) cookie survives.
#[must_use]
pub fn revoked<'a>(cfg: &Config, name: &'static str, path: &'static str) -> Cookie<'a> {
    let mut c =
        Cookie::build(name, "").path(path).http_only(true).max_age(Duration::seconds(0)).finish();
    if !cfg.cookie.domain.is_empty() {
        c.set_domain(cfg.cookie.domain.clone());
    }
    c
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use actix_web::cookie::SameSite;
    use my_fam_tree_config::storage::StorageDriver;
    use my_fam_tree_config::{
        ApiBindConfig, AppEnv, CookieConfig, DatabaseConfig, EmailConfig, JwtConfig, LogConfig,
        LogFormat, MagicLinkConfig, RedisConfig, StorageConfig, WebConfig,
    };

    use super::*;

    fn test_cfg() -> Config {
        Config {
            app_env: AppEnv::Development,
            log: LogConfig { level: "info".into(), format: LogFormat::Pretty },
            api: ApiBindConfig {
                host: "0.0.0.0".into(),
                port: 8080,
                public_url: "http://localhost:8080".into(),
                cors_allowed_origins: "http://localhost:5173".into(),
                enable_docs: false,
                metrics_bind: "0.0.0.0:9090".into(),
            },
            web: WebConfig { public_url: "http://localhost:5173".into() },
            database: DatabaseConfig {
                url: "postgres://t:t@localhost/t".into(),
                max_connections: 4,
                acquire_timeout_seconds: 5,
                statement_timeout_ms: 30_000,
            },
            redis: RedisConfig {
                url: "redis://localhost:6379/0".into(),
                max_connections: 4,
                key_prefix: "t:".into(),
            },
            jwt: JwtConfig {
                private_key: "x".into(),
                private_key_id: "t".into(),
                public_keys: "[]".into(),
                issuer: "iss".into(),
                audience: "aud".into(),
                access_ttl_seconds: 900,
                refresh_ttl_seconds: 86_400,
                refresh_absolute_ttl_seconds: 604_800,
            },
            cookie: CookieConfig {
                domain: String::new(),
                secure: true,
                samesite_access: "Lax".into(),
                samesite_refresh: "Strict".into(),
            },
            magic_link: MagicLinkConfig {
                ttl_seconds: 900,
                invite_ttl_seconds: 1_209_600,
                rate_per_email_per_hour: 10,
                rate_per_ip_per_hour: 100,
            },
            email: EmailConfig {
                dsn: "smtp://localhost:1025".into(),
                from_name: "t".into(),
                from_address: "no-reply@t".into(),
                reply_to: None,
                timeout_seconds: 10,
            },
            storage: StorageConfig {
                driver: StorageDriver::Local,
                bucket: "t".into(),
                region: "us-east-1".into(),
                endpoint_url: None,
                access_key_id: String::new(),
                secret_access_key: String::new(),
                force_path_style: false,
            },
        }
    }

    #[test]
    fn access_cookie_carries_flags_and_root_path() {
        let cfg = test_cfg();
        let c = access_cookie(&cfg, "tok".into());
        assert_eq!(c.name(), ACCESS_COOKIE);
        assert_eq!(c.value(), "tok");
        assert_eq!(c.http_only(), Some(true));
        assert_eq!(c.secure(), Some(true));
        assert_eq!(c.same_site(), Some(SameSite::Lax));
        assert_eq!(c.path(), Some("/"));
        assert_eq!(c.max_age().map(actix_web::cookie::time::Duration::whole_seconds), Some(900));
    }

    #[test]
    fn refresh_cookie_is_scoped_to_refresh_path_with_strict_samesite() {
        let cfg = test_cfg();
        let c = refresh_cookie(&cfg, "rtok".into());
        assert_eq!(c.name(), REFRESH_COOKIE);
        assert_eq!(c.path(), Some(REFRESH_COOKIE_PATH));
        assert_eq!(c.same_site(), Some(SameSite::Strict));
        assert_eq!(c.http_only(), Some(true));
        assert_eq!(c.max_age().map(actix_web::cookie::time::Duration::whole_seconds), Some(86_400));
    }

    #[test]
    fn revoked_cookie_has_zero_max_age_and_named_path() {
        let cfg = test_cfg();
        let c = revoked(&cfg, ACCESS_COOKIE, "/");
        assert_eq!(c.name(), ACCESS_COOKIE);
        assert_eq!(c.value(), "");
        assert_eq!(c.path(), Some("/"));
        assert_eq!(c.http_only(), Some(true));
        assert_eq!(c.max_age().map(actix_web::cookie::time::Duration::whole_seconds), Some(0));
    }

    #[test]
    fn revoked_cookie_inherits_domain_so_browsers_actually_drop_it() {
        let mut cfg = test_cfg();
        cfg.cookie.domain = ".my-fam-tree.docker".into();
        let c = revoked(&cfg, ACCESS_COOKIE, "/");
        assert_eq!(c.domain(), Some(".my-fam-tree.docker"));
    }

    #[test]
    fn cookie_domain_when_set_propagates_to_cookie() {
        let mut cfg = test_cfg();
        cfg.cookie.domain = ".my-fam-tree.docker".into();
        let c = access_cookie(&cfg, "tok".into());
        assert_eq!(c.domain(), Some(".my-fam-tree.docker"));
    }

    #[test]
    fn samesite_parser_recognises_strict_lax_none() {
        assert_eq!(parse_samesite("Strict"), SameSite::Strict);
        assert_eq!(parse_samesite("None"), SameSite::None);
        assert_eq!(parse_samesite("Lax"), SameSite::Lax);
        assert_eq!(parse_samesite("bogus"), SameSite::Lax);
    }
}
