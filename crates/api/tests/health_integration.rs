//! Integration test for the full middleware stack — env vars -> Config -> App -> request.
//! Uses `figment::Jail` to mutate env vars safely (the workspace forbids `unsafe_code`).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    // `figment::Jail::expect_with` requires a closure returning `figment::Result<()>`,
    // whose `Err` variant is large by design.
    clippy::result_large_err
)]

use actix_web::test as actix_test;
use figment::Jail;
use my_family_api::{Config, build_app};

#[test]
fn health_through_full_stack() {
    Jail::expect_with(|jail| {
        for (k, v) in [
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
        ] {
            jail.set_env(k, v);
        }
        let cfg = Config::load_from_env().expect("config");

        // Build a single-thread actix runtime to drive the request.
        let sys = actix_web::rt::System::new();
        sys.block_on(async move {
            let app = actix_test::init_service(build_app(&cfg)).await;
            let req = actix_test::TestRequest::get().uri("/api/v1/health").to_request();
            let res = actix_test::call_service(&app, req).await;
            assert_eq!(res.status(), 200);
            let rid = res.headers().get("x-request-id").unwrap().to_str().unwrap();
            assert!(!rid.is_empty(), "request id missing");
            let body = actix_test::read_body(res).await;
            let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
            assert_eq!(v["data"]["status"], "ok");
        });
        Ok(())
    });
}
