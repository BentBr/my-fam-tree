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

// ---------------------------------------------------------------------------
// Adversarial / error-path coverage.
//
// These tests share `ephemeral_stack()` but each spins its own Postgres +
// Redis pair so they can run in parallel without crosstalk. They exercise
// the error arms of the auth + family + invite handlers — the happy path is
// already covered by the test above.
// ---------------------------------------------------------------------------

/// Call the service tolerantly: middlewares such as `AuthMiddleware::required`
/// surface auth failures via `Err(actix_web::Error)` rather than
/// `Ok(ServiceResponse)` (see `notes/auth-middleware-panic-recovery`), so
/// `test::call_service` would panic. We catch the error and rebuild the
/// `ServiceResponse` via `HttpResponse::from_error`, matching what the actix
/// server does for a real client.
#[allow(clippy::future_not_send)]
async fn try_call<S, B>(
    app: &S,
    req: actix_http::Request,
) -> actix_web::dev::ServiceResponse<actix_web::body::EitherBody<B>>
where
    S: actix_web::dev::Service<
            actix_http::Request,
            Response = actix_web::dev::ServiceResponse<B>,
            Error = actix_web::Error,
        >,
    B: actix_web::body::MessageBody,
{
    match app.call(req).await {
        Ok(resp) => resp.map_into_left_body(),
        Err(err) => {
            // No `ServiceRequest` is available here — build a stand-alone
            // response and pair it with a fresh empty test request so the
            // returned `ServiceResponse` is shaped like the real thing.
            let resp = actix_web::HttpResponse::from_error(err).map_into_right_body();
            let stub = test::TestRequest::default().to_http_request();
            actix_web::dev::ServiceResponse::new(stub, resp)
        }
    }
}

/// Sign in: request a magic link, drain the captured email, consume the
/// token, return both `access` and `refresh` cookie values so the caller can
/// attach them to subsequent requests.
///
/// Generic over the service type so we can pass it the value returned by
/// `test::init_service(...)` without naming its very long concrete type.
///
/// `actix-web`'s test service is single-threaded (`!Send`), so this future
/// inherits the same constraint — perfectly fine inside `#[tokio::test]`
/// runners, but clippy's `future_not_send` lint trips because the helper is
/// `pub`-shaped to the test crate. The lint is irrelevant for test-only code.
#[allow(clippy::future_not_send)]
async fn sign_in<S, B>(stack: &TestStack, app: &S, email: &str) -> (String, String)
where
    S: actix_web::dev::Service<
            actix_http::Request,
            Response = actix_web::dev::ServiceResponse<B>,
            Error = actix_web::Error,
        >,
    B: actix_web::body::MessageBody,
{
    let req = test::TestRequest::post()
        .uri("/api/v1/auth/magic-link")
        .set_json(serde_json::json!({ "email": email }))
        .to_request();
    let res = test::call_service(app, req).await;
    assert_eq!(res.status(), 200, "magic-link request should succeed for {email}");
    let captured = stack.fake_email.drain();
    let last = captured.last().expect("magic-link email captured");
    let token = extract_token_from_link(&last.text_body);

    let req = test::TestRequest::post()
        .uri("/api/v1/auth/consume")
        .set_json(serde_json::json!({ "token": token }))
        .to_request();
    let res = test::call_service(app, req).await;
    assert_eq!(res.status(), 200, "consume should succeed for {email}");
    let access = res.response().cookies().find(|c| c.name() == "access").expect("access cookie");
    let refresh = res.response().cookies().find(|c| c.name() == "refresh").expect("refresh cookie");
    (access.value().to_string(), refresh.value().to_string())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn magic_link_rejects_invalid_email() {
    let stack = ephemeral_stack().await;
    let app = test::init_service(build_app(stack.state.clone(), None)).await;
    let req = test::TestRequest::post()
        .uri("/api/v1/auth/magic-link")
        .set_json(serde_json::json!({ "email": "nope" }))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 422, "syntactically broken email must be rejected");
    let body: serde_json::Value = test::read_body_json(res).await;
    assert_eq!(body["code"], "validation_failed");
    assert_eq!(body["fields"][0]["code"], "validation.email_invalid");
    assert_eq!(body["fields"][0]["path"], "/email");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn consume_rejects_empty_unknown_and_replayed_tokens() {
    let stack = ephemeral_stack().await;
    let app = test::init_service(build_app(stack.state.clone(), None)).await;

    // Empty token — short-circuits before any DB hit.
    let req = test::TestRequest::post()
        .uri("/api/v1/auth/consume")
        .set_json(serde_json::json!({ "token": "" }))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 401);
    let body: serde_json::Value = test::read_body_json(res).await;
    assert_eq!(body["code"], "auth_magic_link_invalid");

    // Unknown opaque token — the hash exists, just not in the table.
    let req = test::TestRequest::post()
        .uri("/api/v1/auth/consume")
        .set_json(serde_json::json!({ "token": "deadbeefdeadbeefdeadbeefdeadbeef" }))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 401);
    let body: serde_json::Value = test::read_body_json(res).await;
    assert_eq!(body["code"], "auth_magic_link_invalid");

    // Replay: consume a real token twice. Issue a magic link, consume once,
    // then consume the same token again — the second call must 401.
    let req = test::TestRequest::post()
        .uri("/api/v1/auth/magic-link")
        .set_json(serde_json::json!({ "email": "replay@example.com" }))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 200);
    let captured = stack.fake_email.drain();
    let token = extract_token_from_link(&captured.last().expect("email").text_body);

    let first = test::TestRequest::post()
        .uri("/api/v1/auth/consume")
        .set_json(serde_json::json!({ "token": token.clone() }))
        .to_request();
    let res = test::call_service(&app, first).await;
    assert_eq!(res.status(), 200, "first consume should succeed");

    let second = test::TestRequest::post()
        .uri("/api/v1/auth/consume")
        .set_json(serde_json::json!({ "token": token }))
        .to_request();
    let res = test::call_service(&app, second).await;
    assert_eq!(res.status(), 401, "replayed token must be rejected");
    let body: serde_json::Value = test::read_body_json(res).await;
    assert_eq!(body["code"], "auth_magic_link_invalid");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn refresh_rejects_missing_and_bogus_cookies() {
    let stack = ephemeral_stack().await;
    let app = test::init_service(build_app(stack.state.clone(), None)).await;

    // No cookie at all.
    let req = test::TestRequest::post().uri("/api/v1/auth/refresh").to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 401);
    let body: serde_json::Value = test::read_body_json(res).await;
    assert_eq!(body["code"], "auth_refresh_invalid");

    // Garbage cookie — hashes to something the DB doesn't know about.
    let req = test::TestRequest::post()
        .uri("/api/v1/auth/refresh")
        .cookie(Cookie::new("refresh", "garbage-not-a-real-token"))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 401);
    let body: serde_json::Value = test::read_body_json(res).await;
    assert_eq!(body["code"], "auth_refresh_invalid");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn logout_without_session_returns_unauthenticated() {
    // /auth/logout sits behind AuthMiddleware::required, so calling it
    // without an access cookie is a 401 — not the idempotent 200 you'd get
    // from a session-less logout endpoint. This pins that behaviour and
    // exercises the middleware's missing-cookie arm.
    let stack = ephemeral_stack().await;
    let app = test::init_service(build_app(stack.state.clone(), None)).await;

    let req = test::TestRequest::post().uri("/api/v1/auth/logout").to_request();
    let res = try_call(&app, req).await;
    assert_eq!(res.status(), 401);
    let body: serde_json::Value = test::read_body_json(res).await;
    assert_eq!(body["code"], "auth_unauthenticated");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn logout_with_session_clears_cookies_and_revokes_refresh() {
    // Sign in, then logout with both cookies. The handler should:
    //  - return 200,
    //  - emit cleared `access` + `refresh` cookies (expired Max-Age),
    //  - revoke the underlying refresh row (verified indirectly: a refresh
    //    call with the same cookie afterwards must 401).
    let stack = ephemeral_stack().await;
    let app = test::init_service(build_app(stack.state.clone(), None)).await;
    let (access, refresh) = sign_in(&stack, &app, "logout-me@example.com").await;

    let req = test::TestRequest::post()
        .uri("/api/v1/auth/logout")
        .cookie(Cookie::new("access", access.clone()))
        .cookie(Cookie::new("refresh", refresh.clone()))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 200);
    let cleared_access = res.response().cookies().find(|c| c.name() == "access").expect("access");
    let cleared_refresh =
        res.response().cookies().find(|c| c.name() == "refresh").expect("refresh");
    assert!(cleared_access.value().is_empty(), "access cookie should be cleared");
    assert!(cleared_refresh.value().is_empty(), "refresh cookie should be cleared");

    // The refresh row must be revoked — a subsequent /auth/refresh 401s.
    let req = test::TestRequest::post()
        .uri("/api/v1/auth/refresh")
        .cookie(Cookie::new("refresh", refresh))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 401);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn create_family_rejects_empty_name() {
    let stack = ephemeral_stack().await;
    let app = test::init_service(build_app(stack.state.clone(), None)).await;
    let (access, _refresh) = sign_in(&stack, &app, "empty-name@example.com").await;

    let req = test::TestRequest::post()
        .uri("/api/v1/families")
        .cookie(Cookie::new("access", access))
        .set_json(serde_json::json!({ "name": "   " }))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 422);
    let body: serde_json::Value = test::read_body_json(res).await;
    assert_eq!(body["fields"][0]["code"], "validation.value_required");
    assert_eq!(body["fields"][0]["path"], "/name");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn invite_rejects_owner_role_and_invalid_email() {
    let stack = ephemeral_stack().await;
    let app = test::init_service(build_app(stack.state.clone(), None)).await;
    let (access, _refresh) = sign_in(&stack, &app, "owner@example.com").await;

    // Create a family so we have a valid family id and Owner membership.
    let req = test::TestRequest::post()
        .uri("/api/v1/families")
        .cookie(Cookie::new("access", access))
        .set_json(serde_json::json!({ "name": "Owners" }))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 200);
    let owner_access = res
        .response()
        .cookies()
        .find(|c| c.name() == "access")
        .expect("rotated access")
        .value()
        .to_string();
    let body: serde_json::Value = test::read_body_json(res).await;
    let family_id = body["data"]["family"]["id"].as_str().expect("family id").to_string();

    // Inviting as owner is disallowed by the validation rule.
    let req = test::TestRequest::post()
        .uri(&format!("/api/v1/families/{family_id}/invites"))
        .cookie(Cookie::new("access", owner_access.clone()))
        .set_json(serde_json::json!({ "email": "guest@example.com", "role": "owner" }))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 422);
    let body: serde_json::Value = test::read_body_json(res).await;
    assert_eq!(body["fields"][0]["code"], "validation.role_invalid");
    assert_eq!(body["fields"][0]["path"], "/role");

    // Inviting with a malformed email also fails validation.
    let req = test::TestRequest::post()
        .uri(&format!("/api/v1/families/{family_id}/invites"))
        .cookie(Cookie::new("access", owner_access))
        .set_json(serde_json::json!({ "email": "not-an-email", "role": "user" }))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 422);
    let body: serde_json::Value = test::read_body_json(res).await;
    assert_eq!(body["fields"][0]["code"], "validation.email_invalid");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn invite_accept_happy_path_and_email_mismatch() {
    // End-to-end invite flow. The accept handler is mark-and-fetch atomic —
    // the invite row is flipped to `accepted_at=now()` BEFORE the
    // signed-in-email check runs, so a wrong-email attempt also burns the
    // invite. We use two separate invites here to cover both arms cleanly:
    //   - invite #1 for user-mismatch@... is accepted by user-c@... -> 422
    //     `validation.invite_email_mismatch`.
    //   - invite #2 for user-b@...        is accepted by user-b@...  -> 200
    //     with the rotated access cookie listing the family as Admin.
    //   - a third accept with a bogus token returns 401 (NotFoundOrAccepted).
    let stack = ephemeral_stack().await;
    let app = test::init_service(build_app(stack.state.clone(), None)).await;

    let (access_a, _refresh_a) = sign_in(&stack, &app, "user-a@example.com").await;
    // Drain the magic-link email so subsequent `last()` calls land on invites.
    stack.fake_email.drain();

    let req = test::TestRequest::post()
        .uri("/api/v1/families")
        .cookie(Cookie::new("access", access_a))
        .set_json(serde_json::json!({ "name": "Alpha" }))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 200);
    let rotated_access_a = res
        .response()
        .cookies()
        .find(|c| c.name() == "access")
        .expect("rotated access")
        .value()
        .to_string();
    let body: serde_json::Value = test::read_body_json(res).await;
    let family_id = body["data"]["family"]["id"].as_str().expect("family id").to_string();

    // Invite #1: addressed to user-mismatch@..., used to drive the email
    // mismatch arm without burning the user-b@... invite.
    let req = test::TestRequest::post()
        .uri(&format!("/api/v1/families/{family_id}/invites"))
        .cookie(Cookie::new("access", rotated_access_a.clone()))
        .set_json(serde_json::json!({ "email": "user-mismatch@example.com", "role": "admin" }))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 200);
    let token_mismatch = {
        let captured = stack.fake_email.drain();
        let mail = captured.last().expect("invite email captured");
        assert_eq!(mail.to_addr, "user-mismatch@example.com");
        extract_token_from_link(&mail.text_body)
    };

    // Invite #2: the one user B will accept.
    let req = test::TestRequest::post()
        .uri(&format!("/api/v1/families/{family_id}/invites"))
        .cookie(Cookie::new("access", rotated_access_a))
        .set_json(serde_json::json!({ "email": "user-b@example.com", "role": "admin" }))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 200);
    let token_b = {
        let captured = stack.fake_email.drain();
        let mail = captured.last().expect("invite email captured");
        assert_eq!(mail.to_addr, "user-b@example.com");
        extract_token_from_link(&mail.text_body)
    };

    // User C tries to accept invite #1 — different email, validation fires.
    let (access_c, _refresh_c) = sign_in(&stack, &app, "user-c@example.com").await;
    let req = test::TestRequest::post()
        .uri("/api/v1/invites/accept")
        .cookie(Cookie::new("access", access_c))
        .set_json(serde_json::json!({ "token": token_mismatch }))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 422);
    let body: serde_json::Value = test::read_body_json(res).await;
    assert_eq!(body["fields"][0]["code"], "validation.invite_email_mismatch");

    // User B signs in and accepts invite #2 — happy path.
    let (access_b, _refresh_b) = sign_in(&stack, &app, "user-b@example.com").await;
    let req = test::TestRequest::post()
        .uri("/api/v1/invites/accept")
        .cookie(Cookie::new("access", access_b))
        .set_json(serde_json::json!({ "token": token_b }))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 200);
    let new_access_value = res
        .response()
        .cookies()
        .find(|c| c.name() == "access")
        .expect("access")
        .value()
        .to_string();
    let body: serde_json::Value = test::read_body_json(res).await;
    assert_eq!(body["data"]["family"]["role"], "admin");

    // A subsequent accept with a bogus token returns 401 — the hash doesn't
    // match any row, so the repo's `NotFoundOrAccepted` arm maps to
    // `MagicLinkInvalid` (401, `auth_magic_link_invalid`).
    let req = test::TestRequest::post()
        .uri("/api/v1/invites/accept")
        .cookie(Cookie::new("access", new_access_value))
        .set_json(serde_json::json!({ "token": "bad-token" }))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 401, "missing/used token must be MagicLinkInvalid");
    let body: serde_json::Value = test::read_body_json(res).await;
    assert_eq!(body["code"], "auth_magic_link_invalid");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn family_operations_require_membership_and_role() {
    // Two users, two families. A owns Alpha, B owns Beta.
    //   - A renaming Beta -> 403 (not a member).
    //   - A inviting to Beta -> 403 (not a member).
    //   - A deleting Beta -> 403 (not a member).
    //   - A renaming Alpha with empty name -> 422.
    //   - GET /families/me with no session -> 401.
    let stack = ephemeral_stack().await;
    let app = test::init_service(build_app(stack.state.clone(), None)).await;

    let (access_a, _r_a) = sign_in(&stack, &app, "user-a2@example.com").await;
    let req = test::TestRequest::post()
        .uri("/api/v1/families")
        .cookie(Cookie::new("access", access_a))
        .set_json(serde_json::json!({ "name": "Alpha" }))
        .to_request();
    let res = test::call_service(&app, req).await;
    let access_a_rot = res
        .response()
        .cookies()
        .find(|c| c.name() == "access")
        .expect("access a")
        .value()
        .to_string();
    let body: serde_json::Value = test::read_body_json(res).await;
    let family_alpha = body["data"]["family"]["id"].as_str().expect("alpha id").to_string();

    let (access_b, _r_b) = sign_in(&stack, &app, "user-b2@example.com").await;
    let req = test::TestRequest::post()
        .uri("/api/v1/families")
        .cookie(Cookie::new("access", access_b))
        .set_json(serde_json::json!({ "name": "Beta" }))
        .to_request();
    let res = test::call_service(&app, req).await;
    let body: serde_json::Value = test::read_body_json(res).await;
    let family_beta = body["data"]["family"]["id"].as_str().expect("beta id").to_string();

    // A tries to rename Beta — 403 NotFamilyMember.
    let req = test::TestRequest::patch()
        .uri(&format!("/api/v1/families/{family_beta}"))
        .cookie(Cookie::new("access", access_a_rot.clone()))
        .set_json(serde_json::json!({ "name": "Stolen" }))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 403);
    let body: serde_json::Value = test::read_body_json(res).await;
    assert_eq!(body["code"], "family_not_member");

    // A tries to invite into Beta — same 403.
    let req = test::TestRequest::post()
        .uri(&format!("/api/v1/families/{family_beta}/invites"))
        .cookie(Cookie::new("access", access_a_rot.clone()))
        .set_json(serde_json::json!({ "email": "x@example.com", "role": "user" }))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 403);

    // A tries to delete Beta — same 403.
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1/families/{family_beta}"))
        .cookie(Cookie::new("access", access_a_rot.clone()))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 403);

    // A renames Alpha with an empty name — 422.
    let req = test::TestRequest::patch()
        .uri(&format!("/api/v1/families/{family_alpha}"))
        .cookie(Cookie::new("access", access_a_rot.clone()))
        .set_json(serde_json::json!({ "name": "   " }))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 422);
    let body: serde_json::Value = test::read_body_json(res).await;
    assert_eq!(body["fields"][0]["code"], "validation.value_required");

    // A actually renames Alpha — happy patch.
    let req = test::TestRequest::patch()
        .uri(&format!("/api/v1/families/{family_alpha}"))
        .cookie(Cookie::new("access", access_a_rot.clone()))
        .set_json(serde_json::json!({ "name": "Alpha Renamed" }))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 200);
    let body: serde_json::Value = test::read_body_json(res).await;
    assert_eq!(body["data"]["name"], "Alpha Renamed");

    // GET /families/me without a session — 401 from AuthMiddleware.
    let req = test::TestRequest::get().uri("/api/v1/families/me").to_request();
    let res = try_call(&app, req).await;
    assert_eq!(res.status(), 401);

    // GET /families/me with A's session — 1 membership.
    let req = test::TestRequest::get()
        .uri("/api/v1/families/me")
        .cookie(Cookie::new("access", access_a_rot.clone()))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 200);
    let body: serde_json::Value = test::read_body_json(res).await;
    assert_eq!(body["data"]["families"].as_array().unwrap().len(), 1);

    // A deletes Alpha — happy path covers the DELETE handler success arm.
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1/families/{family_alpha}"))
        .cookie(Cookie::new("access", access_a_rot))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 200);
}
