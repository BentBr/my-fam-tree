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
        .secure(cfg.cookie_secure)
        .same_site(parse_samesite(&cfg.cookie_samesite_access))
        .path("/")
        .max_age(seconds(cfg.jwt_access_ttl_seconds))
        .finish();
    if !cfg.cookie_domain.is_empty() {
        c.set_domain(cfg.cookie_domain.clone());
    }
    c
}

/// Build the long-lived refresh cookie. Scoped to the `/auth/refresh` path so
/// the browser will never attach it to other endpoints.
#[must_use]
pub fn refresh_cookie<'a>(cfg: &Config, value: String) -> Cookie<'a> {
    let mut c = Cookie::build(REFRESH_COOKIE, value)
        .http_only(true)
        .secure(cfg.cookie_secure)
        .same_site(parse_samesite(&cfg.cookie_samesite_refresh))
        .path(REFRESH_COOKIE_PATH)
        .max_age(seconds(cfg.jwt_refresh_ttl_seconds))
        .finish();
    if !cfg.cookie_domain.is_empty() {
        c.set_domain(cfg.cookie_domain.clone());
    }
    c
}

/// Build a zero-value, zero-age cookie used by `/auth/logout` to instruct the
/// browser to drop the named cookie at the given path.
#[must_use]
pub fn revoked<'a>(name: &'static str, path: &'static str) -> Cookie<'a> {
    Cookie::build(name, "").path(path).http_only(true).max_age(Duration::seconds(0)).finish()
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use actix_web::cookie::SameSite;

    use super::*;
    use crate::config::{AppEnv, LogFormat};

    fn test_cfg() -> Config {
        Config {
            app_env: AppEnv::Development,
            log_format: LogFormat::Pretty,
            rust_log: "info".into(),
            api_host: "0.0.0.0".into(),
            api_port: 8080,
            api_public_url: "http://localhost:8080".into(),
            web_public_url: "http://localhost:5173".into(),
            cors_allowed_origins: "http://localhost:5173".into(),
            api_enable_docs: false,
            api_metrics_bind: "0.0.0.0:9090".into(),
            database_url: "postgres://t:t@localhost/t".into(),
            database_max_connections: 4,
            database_acquire_timeout_seconds: 5,
            database_statement_timeout_ms: 30_000,
            redis_url: "redis://localhost:6379/0".into(),
            redis_max_connections: 4,
            redis_key_prefix: "t:".into(),
            jwt_private_key: "x".into(),
            jwt_private_key_id: "t".into(),
            jwt_public_keys: "[]".into(),
            jwt_issuer: "iss".into(),
            jwt_audience: "aud".into(),
            jwt_access_ttl_seconds: 900,
            jwt_refresh_ttl_seconds: 86_400,
            jwt_refresh_absolute_ttl_seconds: 604_800,
            cookie_domain: String::new(),
            cookie_secure: true,
            cookie_samesite_access: "Lax".into(),
            cookie_samesite_refresh: "Strict".into(),
            magic_link_ttl_seconds: 900,
            invite_ttl_seconds: 1_209_600,
            magic_link_rate_per_email_per_hour: 10,
            magic_link_rate_per_ip_per_hour: 100,
            email_dsn: "smtp://localhost:1025".into(),
            email_from_name: "t".into(),
            email_from_address: "no-reply@t".into(),
            email_reply_to: None,
            email_timeout_seconds: 10,
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
        let c = revoked(ACCESS_COOKIE, "/");
        assert_eq!(c.name(), ACCESS_COOKIE);
        assert_eq!(c.value(), "");
        assert_eq!(c.path(), Some("/"));
        assert_eq!(c.http_only(), Some(true));
        assert_eq!(c.max_age().map(actix_web::cookie::time::Duration::whole_seconds), Some(0));
    }

    #[test]
    fn cookie_domain_when_set_propagates_to_cookie() {
        let mut cfg = test_cfg();
        cfg.cookie_domain = ".my-family.docker".into();
        let c = access_cookie(&cfg, "tok".into());
        assert_eq!(c.domain(), Some(".my-family.docker"));
    }

    #[test]
    fn samesite_parser_recognises_strict_lax_none() {
        assert_eq!(parse_samesite("Strict"), SameSite::Strict);
        assert_eq!(parse_samesite("None"), SameSite::None);
        assert_eq!(parse_samesite("Lax"), SameSite::Lax);
        assert_eq!(parse_samesite("bogus"), SameSite::Lax);
    }
}
