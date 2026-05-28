//! Shared test scaffolding for the API integration tests.
//!
//! Cargo treats every `.rs` file directly under `tests/` as its own
//! integration-test binary. Helpers live under `tests/common/mod.rs` so
//! each test file can `mod common;` them in without Cargo trying to
//! compile this module as a standalone binary.
//!
//! Each downstream test file pulls a different subset of these helpers,
//! so `#![allow(dead_code)]` is needed to keep clippy quiet across all
//! integration crates.

#![allow(dead_code)]
#![allow(unreachable_pub)]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use std::sync::Arc;
use std::time::Duration;

use actix_web::test;
use ed25519_dalek::SigningKey;
use ed25519_dalek::pkcs8::spki::der::pem::LineEnding;
use ed25519_dalek::pkcs8::{EncodePrivateKey, EncodePublicKey};
use my_family_api::auth::{JwtIssuer, JwtKeyset};
use my_family_api::{AppEnv, AppState, Config, LogFormat};
use my_family_cache::{RedisPool, RedisRateLimiter};
use my_family_email::FakeEmailSender;
use my_family_persistence::{
    Database, PgAuditLogRepo, PgEmailOutboxRepo, PgFamilyInviteRepo, PgFamilyMembershipRepo,
    PgFamilyRepo, PgHealthRepo, PgMagicLinkRepo, PgOwnerTransferRepo, PgParentLinkRepo,
    PgPartnershipRepo, PgPersonContactRepo, PgPersonFavouriteRepo, PgPersonRepo,
    PgRefreshTokenRepo, PgReminderDigestRepo, PgReminderPrefsRepo, PgUserRepo,
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
pub struct TestStack {
    pub state: AppState,
    pub fake_email: Arc<FakeEmailSender>,
    pub _pg: ContainerAsync<Postgres>,
    pub _redis: ContainerAsync<Redis>,
}

pub async fn ephemeral_stack() -> TestStack {
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
        invites: Arc::new(PgFamilyInviteRepo::new(pool.clone())),
        persons: Arc::new(PgPersonRepo::new(pool.clone())),
        parent_links: Arc::new(PgParentLinkRepo::new(pool.clone())),
        partnerships: Arc::new(PgPartnershipRepo::new(pool.clone())),
        contacts: Arc::new(PgPersonContactRepo::new(pool.clone())),
        favourites: Arc::new(PgPersonFavouriteRepo::new(pool.clone())),
        owner_transfers: Arc::new(PgOwnerTransferRepo::new(pool.clone())),
        reminder_prefs: Arc::new(PgReminderPrefsRepo::new(pool.clone())),
        reminder_digests: Arc::new(PgReminderDigestRepo::new(pool.clone())),
        health: Arc::new(PgHealthRepo::new(pool.clone())),
        audit: Arc::new(PgAuditLogRepo::new(pool.clone())),
        email: fake_email.clone(),
        outbox: Arc::new(PgEmailOutboxRepo::new(pool)),
        rate_limiter: Arc::new(RedisRateLimiter::new(redis_pool.clone())),
        redis: redis_pool,
        jwt_issuer: Arc::new(issuer),
    };

    TestStack { state, fake_email, _pg: pg, _redis: redis_container }
}

/// Drain the `email_outbox` synchronously into the `FakeEmailSender` —
/// mirrors what the worker's outbox poller does in prod/e2e. Used by
/// `sign_in` (and any future helper that triggers an email-producing
/// handler) so existing tests that read `stack.fake_email.drain()` keep
/// working transparently.
#[allow(clippy::future_not_send)]
pub async fn drain_outbox_now(stack: &TestStack) {
    use my_family_email::OutboundEmail;
    let now = chrono::Utc::now();
    while let Some(row) = stack.state.outbox.claim_next_due(now).await.expect("claim outbox") {
        let email = OutboundEmail {
            to_addr: row.to_addr.clone(),
            to_name: None,
            subject: row.subject.clone(),
            text_body: row.text_body.clone(),
            html_body: row.html_body.clone(),
        };
        stack.state.email.send(email).await.expect("send outbox row");
        stack.state.outbox.mark_sent(row.id, chrono::Utc::now()).await.expect("mark sent");
    }
}

/// Pull the magic-link token out of the email's plain-text body. The template
/// formats the link as `{web_public_url}/auth/consume?token={token}`.
pub fn extract_token_from_link(body: &str) -> String {
    let after = body.split("token=").nth(1).expect("token= present");
    after.split(|c: char| c.is_whitespace() || c == '"').next().expect("token chars").to_string()
}

/// Call the service tolerantly: middlewares such as `AuthMiddleware::required`
/// surface auth failures via `Err(actix_web::Error)` rather than
/// `Ok(ServiceResponse)` (see `notes/auth-middleware-panic-recovery`), so
/// `test::call_service` would panic. We catch the error and rebuild the
/// `ServiceResponse` via `HttpResponse::from_error`, matching what the actix
/// server does for a real client.
#[allow(clippy::future_not_send)]
pub async fn try_call<S, B>(
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
pub async fn sign_in<S, B>(stack: &TestStack, app: &S, email: &str) -> (String, String)
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
    // The magic-link handler now writes to the email_outbox instead of
    // calling EmailSender::send() inline — drain the outbox synchronously
    // so the FakeEmailSender captures the email (the real worker drains
    // it in prod / e2e).
    drain_outbox_now(stack).await;
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

/// Create a family for the caller (becomes Owner) and return the rotated
/// access cookie plus the new `family_id` as a string.
///
/// Mirrors what `POST /api/v1/families` does in the real client: the
/// response sets a fresh `access` cookie reflecting the new Owner membership,
/// and the response body's `data.family.id` carries the UUID. Test code
/// then attaches `X-Family-Id` to subsequent requests against the
/// persons / parent-links / partnerships / relationships scope.
#[allow(clippy::future_not_send)]
pub async fn create_family<S, B>(app: &S, access: &str, name: &str) -> (String, String)
where
    S: actix_web::dev::Service<
            actix_http::Request,
            Response = actix_web::dev::ServiceResponse<B>,
            Error = actix_web::Error,
        >,
    B: actix_web::body::MessageBody,
{
    let req = test::TestRequest::post()
        .uri("/api/v1/families")
        .cookie(actix_web::cookie::Cookie::new("access", access.to_string()))
        .set_json(serde_json::json!({ "name": name }))
        .to_request();
    let res = test::call_service(app, req).await;
    assert_eq!(res.status(), 200, "create family should succeed for {name}");
    let new_access = res
        .response()
        .cookies()
        .find(|c| c.name() == "access")
        .expect("rotated access cookie")
        .value()
        .to_string();
    let body: serde_json::Value = test::read_body_json(res).await;
    let family_id = body["data"]["family"]["id"].as_str().expect("family id").to_string();
    (new_access, family_id)
}

// ---------------------------------------------------------------------------
// Three-role family setup (owner + admin + user), shared by the members,
// roles-authz, and similar matrix tests. Memberships are inserted directly
// via the repos; callers then `sign_in` so the JWT's `families` claim mirrors
// the DB (which is all `require_role` ever sees).
// ---------------------------------------------------------------------------

/// Sign `email` in (provisions the `users` row) and return its `UserId`. The
/// session cookies are discarded — callers re-sign-in after memberships exist.
#[allow(clippy::future_not_send)]
pub async fn provision_user<S, B>(
    stack: &TestStack,
    app: &S,
    email: &str,
) -> my_family_domain::UserId
where
    S: actix_web::dev::Service<
            actix_http::Request,
            Response = actix_web::dev::ServiceResponse<B>,
            Error = actix_web::Error,
        >,
    B: actix_web::body::MessageBody,
{
    let _ = sign_in(stack, app, email).await;
    stack.state.users.find_by_email(email).await.expect("user lookup").expect("user exists").id
}

/// Insert (or update) a membership row directly via the repo.
#[allow(clippy::future_not_send)]
pub async fn ensure_membership(
    state: &my_family_api::AppState,
    family_id: my_family_domain::FamilyId,
    user_id: my_family_domain::UserId,
    role: my_family_domain::Role,
) {
    if state.memberships.find(family_id, user_id).await.expect("find").is_some() {
        state.memberships.set_role(family_id, user_id, role).await.expect("set_role");
    } else {
        state.memberships.insert(family_id, user_id, role).await.expect("insert");
    }
}

/// A freshly seeded family with one member at each role.
pub struct ThreeRoleFamily {
    pub family_id: my_family_domain::FamilyId,
    pub owner_email: String,
    pub admin_email: String,
    pub user_email: String,
    pub admin_id: my_family_domain::UserId,
    pub user_id: my_family_domain::UserId,
}

/// Seed a fresh family with an owner, an admin and a user. `stamp` keeps the
/// emails unique across parallel tests.
#[allow(clippy::future_not_send)]
pub async fn seed_three_role_family<S, B>(
    stack: &TestStack,
    app: &S,
    stamp: u128,
) -> ThreeRoleFamily
where
    S: actix_web::dev::Service<
            actix_http::Request,
            Response = actix_web::dev::ServiceResponse<B>,
            Error = actix_web::Error,
        >,
    B: actix_web::body::MessageBody,
{
    let owner_email = format!("roles-owner-{stamp}@example.com");
    let admin_email = format!("roles-admin-{stamp}@example.com");
    let user_email = format!("roles-user-{stamp}@example.com");

    let (owner_access, _r) = sign_in(stack, app, &owner_email).await;
    let (_owner_access, family_id_str) =
        create_family(app, &owner_access, &format!("RolesFam-{stamp}")).await;
    let family_id = my_family_domain::FamilyId::from_uuid(family_id_str.parse().expect("uuid"));

    let admin_id = provision_user(stack, app, &admin_email).await;
    let user_id = provision_user(stack, app, &user_email).await;
    ensure_membership(&stack.state, family_id, admin_id, my_family_domain::Role::Admin).await;
    ensure_membership(&stack.state, family_id, user_id, my_family_domain::Role::User).await;

    ThreeRoleFamily { family_id, owner_email, admin_email, user_email, admin_id, user_id }
}

/// Sign `email` in and return just the access-cookie value (JWT now carries
/// the member's current role for the seeded family).
#[allow(clippy::future_not_send)]
pub async fn fresh_access<S, B>(stack: &TestStack, app: &S, email: &str) -> String
where
    S: actix_web::dev::Service<
            actix_http::Request,
            Response = actix_web::dev::ServiceResponse<B>,
            Error = actix_web::Error,
        >,
    B: actix_web::body::MessageBody,
{
    let (access, _r) = sign_in(stack, app, email).await;
    access
}
