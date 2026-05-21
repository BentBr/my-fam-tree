//! End-to-end auth + family flow against ephemeral Postgres + Redis containers
//! plus a `FakeEmailSender`.
//!
//! Verifies the full chain:
//!
//! 1. `POST /auth/magic-link` creates the user, persists the hashed token,
//!    and queues an email.
//! 2. `POST /auth/consume` exchanges the magic-link token for an `access`
//!    cookie + `refresh` cookie + JSON claims payload.
//! 3. `GET /auth/me` echoes the verified claims.
//! 4. `POST /families` mints a fresh access cookie that reflects the new
//!    `Owner` membership.
//! 5. `GET /auth/me` with the new cookie sees the family.
//! 6. `POST /auth/refresh` rotates both cookies.
//!
//! The container handles are owned by the test (NOT `Box::leak`-ed) so their
//! `Drop` impls trigger the testcontainers reaper. Without that we'd accumulate
//! orphan Postgres + Redis containers on every run.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::shadow_unrelated,
    clippy::shadow_reuse,
    clippy::shadow_same
)]

use std::sync::Arc;
use std::time::Duration;

use actix_web::cookie::Cookie;
use actix_web::test;
use ed25519_dalek::SigningKey;
use ed25519_dalek::pkcs8::spki::der::pem::LineEnding;
use ed25519_dalek::pkcs8::{EncodePrivateKey, EncodePublicKey};
use my_family_api::auth::{JwtIssuer, JwtKeyset};
use my_family_api::{AppEnv, AppState, Config, LogFormat, build_app};
use my_family_cache::{RedisPool, RedisRateLimiter};
use my_family_email::FakeEmailSender;
use my_family_persistence::{
    Database, PgFamilyInviteRepo, PgFamilyMembershipRepo, PgFamilyRepo, PgMagicLinkRepo,
    PgRefreshTokenRepo, PgUserRepo,
};
use rand::rngs::OsRng;
use testcontainers::ContainerAsync;
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::postgres::Postgres;
use testcontainers_modules::redis::Redis;

/// Bundles the `AppState`, the `FakeEmailSender` (so tests can drain captured
/// outbound mail), and the postgres + redis container handles. Holding the
/// `ContainerAsync` values in the test scope ensures their `Drop` impls run
/// when the test finishes — the testcontainers reaper then stops + removes
/// the docker containers. `Box::leak` would skip that and leak containers.
struct TestStack {
    state: AppState,
    fake_email: Arc<FakeEmailSender>,
    _pg: ContainerAsync<Postgres>,
    _redis: ContainerAsync<Redis>,
}

async fn ephemeral_stack() -> TestStack {
    let pg = Postgres::default()
        .with_db_name("t")
        .with_user("t")
        .with_password("t")
        .start()
        .await
        .expect("start pg");
    let pg_port = pg.get_host_port_ipv4(5432_u16).await.expect("pg port");
    let db_url = format!("postgres://t:t@127.0.0.1:{pg_port}/t");

    let redis_container = Redis::default().start().await.expect("start redis");
    let redis_port = redis_container.get_host_port_ipv4(6379_u16).await.expect("redis port");
    let redis_url = format!("redis://127.0.0.1:{redis_port}/0");

    // Wait for Postgres to accept connections — testcontainers reports
    // readiness on log scan but the JDBC port can lag a few ms behind.
    let mut connected: Option<Database> = None;
    for _ in 0_u8..40_u8 {
        if let Ok(db) = Database::connect(&db_url, 2, Duration::from_secs(1), 30_000).await {
            connected = Some(db);
            break;
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
    let db = connected.expect("postgres never accepted connections");
    sqlx::migrate!("../../migrations").run(db.pool()).await.expect("migrate");

    // Ephemeral Ed25519 keypair so the JWT issuer is real but isolated.
    let signing = SigningKey::generate(&mut OsRng);
    let priv_pem = signing.to_pkcs8_pem(LineEnding::LF).unwrap().to_string();
    let pub_pem = signing.verifying_key().to_public_key_pem(LineEnding::LF).unwrap();
    let public_json =
        serde_json::json!([{"kid": "t", "public_pem": pub_pem.trim_end()}]).to_string();
    let keys = JwtKeyset::load(&priv_pem, "t", &public_json).expect("load keys");
    let issuer = JwtIssuer::new(keys, "iss".into(), "aud".into(), 900);

    let cfg = Config {
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
        database_url: db_url.clone(),
        database_max_connections: 4,
        database_acquire_timeout_seconds: 5,
        database_statement_timeout_ms: 30_000,
        redis_url: redis_url.clone(),
        redis_max_connections: 4,
        redis_key_prefix: "t:".into(),
        jwt_private_key: priv_pem,
        jwt_private_key_id: "t".into(),
        jwt_public_keys: public_json,
        jwt_issuer: "iss".into(),
        jwt_audience: "aud".into(),
        jwt_access_ttl_seconds: 900,
        jwt_refresh_ttl_seconds: 86_400,
        jwt_refresh_absolute_ttl_seconds: 604_800,
        cookie_domain: String::new(),
        cookie_secure: false,
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
    };

    let fake_email = Arc::new(FakeEmailSender::new());
    let pool = db.pool().clone();
    let redis_pool = RedisPool::build(&redis_url, 4, "t:").expect("redis pool");
    let state = AppState {
        cfg: Arc::new(cfg),
        users: Arc::new(PgUserRepo::new(pool.clone())),
        magic_links: Arc::new(PgMagicLinkRepo::new(pool.clone())),
        refresh_tokens: Arc::new(PgRefreshTokenRepo::new(pool.clone())),
        families: Arc::new(PgFamilyRepo::new(pool.clone())),
        memberships: Arc::new(PgFamilyMembershipRepo::new(pool.clone())),
        invites: Arc::new(PgFamilyInviteRepo::new(pool)),
        email: fake_email.clone(),
        rate_limiter: Arc::new(RedisRateLimiter::new(redis_pool.clone())),
        redis: redis_pool,
        jwt_issuer: Arc::new(issuer),
    };

    TestStack { state, fake_email, _pg: pg, _redis: redis_container }
}

/// Pull the magic-link token out of the email's plain-text body. The template
/// formats the link as `{web_public_url}/auth/consume?token={token}`.
fn extract_token_from_link(body: &str) -> String {
    let after = body.split("token=").nth(1).expect("token= present");
    after.split(|c: char| c.is_whitespace() || c == '"').next().expect("token chars").to_string()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn magic_link_then_consume_then_me_then_create_family_then_refresh() {
    let stack = ephemeral_stack().await;
    let app = test::init_service(build_app(stack.state.clone(), None)).await;

    // 1. Request a magic link.
    let req = test::TestRequest::post()
        .uri("/api/v1/auth/magic-link")
        .set_json(serde_json::json!({ "email": "anna@example.com" }))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 200, "magic-link request should succeed");

    // 2. Grab the email and extract the opaque token.
    let captured = stack.fake_email.drain();
    assert_eq!(captured.len(), 1, "exactly one magic-link email expected");
    let token = extract_token_from_link(&captured[0].text_body);
    assert!(!token.is_empty(), "extracted token must be non-empty");

    // 3. Consume — sets both cookies and returns the claims payload.
    let req = test::TestRequest::post()
        .uri("/api/v1/auth/consume")
        .set_json(serde_json::json!({ "token": token }))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 200, "consume should succeed");
    let access = res.response().cookies().find(|c| c.name() == "access").expect("access cookie");
    let access_value = access.value().to_string();
    let refresh = res.response().cookies().find(|c| c.name() == "refresh").expect("refresh cookie");
    let refresh_value = refresh.value().to_string();

    // 4. /auth/me echoes the freshly minted session.
    let req = test::TestRequest::get()
        .uri("/api/v1/auth/me")
        .cookie(Cookie::new("access", access_value.clone()))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 200);
    let body: serde_json::Value = test::read_body_json(res).await;
    assert_eq!(body["data"]["email"], "anna@example.com");
    assert_eq!(body["data"]["families"].as_array().unwrap().len(), 0);

    // 5. Create a family — handler should reissue the access cookie so the
    //    new Owner membership is immediately visible.
    let req = test::TestRequest::post()
        .uri("/api/v1/families")
        .cookie(Cookie::new("access", access_value))
        .set_json(serde_json::json!({ "name": "Müller" }))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 200);
    let new_access =
        res.response().cookies().find(|c| c.name() == "access").expect("new access cookie");
    let new_access_value = new_access.value().to_string();
    let body: serde_json::Value = test::read_body_json(res).await;
    assert_eq!(body["data"]["family"]["name"], "Müller");
    assert_eq!(body["data"]["claims"]["families"].as_array().unwrap().len(), 1);

    // 6. /auth/me with the rotated cookie reflects the membership.
    let req = test::TestRequest::get()
        .uri("/api/v1/auth/me")
        .cookie(Cookie::new("access", new_access_value))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 200);
    let body: serde_json::Value = test::read_body_json(res).await;
    assert_eq!(body["data"]["families"][0]["role"], "owner");

    // 7. Refresh round-trip: the refresh cookie path is /api/v1/auth/refresh,
    //    so the test cookie passes through fine.
    let req = test::TestRequest::post()
        .uri("/api/v1/auth/refresh")
        .cookie(Cookie::new("refresh", refresh_value))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 200);
    assert!(res.response().cookies().any(|c| c.name() == "access"));
    assert!(res.response().cookies().any(|c| c.name() == "refresh"));
}
