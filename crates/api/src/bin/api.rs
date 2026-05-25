//! Production `api` binary: loads config, initializes tracing, constructs
//! `AppState` (DB pool, Redis pool, repos, JWT issuer, email sender, rate
//! limiter), and serves `build_app(state)`.

use std::sync::Arc;
use std::time::Duration;

use actix_web::HttpServer;
use anyhow::Context;
use my_family_api::auth::{JwtIssuer, JwtKeyset};
use my_family_api::{ApiDoc, AppEnv, AppState, Config, build_app, init_tracing};
use my_family_cache::{RedisPool, RedisRateLimiter};
use my_family_email::SmtpSender;
use my_family_persistence::{
    Database, PgAuditLogRepo, PgFamilyInviteRepo, PgFamilyMembershipRepo, PgFamilyRepo,
    PgMagicLinkRepo, PgParentLinkRepo, PgPartnershipRepo, PgPersonContactRepo, PgPersonRepo,
    PgRefreshTokenRepo, PgUserRepo,
};

#[actix_web::main]
async fn main() -> anyhow::Result<()> {
    if std::env::var("APP_ENV").as_deref() == Ok("development") {
        // `.env` is optional in development; ignore both missing-file and parse
        // errors here because tracing isn't initialized yet and the binary
        // lints forbid `eprintln!`/`println!`. Any real misconfiguration will
        // surface from `Config::load_from_env` below.
        let _ = dotenvy::dotenv();
    }

    let cfg = Config::load_from_env().context("load config from environment")?;

    init_tracing(cfg.log_format, &cfg.rust_log);

    tracing::info!(
        app_env = ?cfg.app_env,
        host = %cfg.api_host,
        port = cfg.api_port,
        "starting my-family api",
    );

    let db = Database::connect(
        &cfg.database_url,
        cfg.database_max_connections,
        Duration::from_secs(cfg.database_acquire_timeout_seconds),
        cfg.database_statement_timeout_ms,
    )
    .await
    .context("connect postgres pool")?;

    let redis = RedisPool::build(&cfg.redis_url, cfg.redis_max_connections, &cfg.redis_key_prefix)
        .context("build redis pool")?;
    redis.ping().await.context("ping redis")?;

    let email = SmtpSender::from_dsn(
        &cfg.email_dsn,
        &cfg.email_from_name,
        &cfg.email_from_address,
        cfg.email_reply_to.as_deref(),
        cfg.email_timeout_seconds,
    )
    .context("build SMTP sender")?;

    let keyset =
        JwtKeyset::load(&cfg.jwt_private_key, &cfg.jwt_private_key_id, &cfg.jwt_public_keys)
            .context("load JWT keyset")?;
    let jwt_issuer = JwtIssuer::new(
        keyset,
        cfg.jwt_issuer.clone(),
        cfg.jwt_audience.clone(),
        i64::try_from(cfg.jwt_access_ttl_seconds).unwrap_or(i64::MAX),
    );

    let pool = db.pool().clone();
    let state = AppState {
        cfg: Arc::new(cfg.clone()),
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
        audit: Arc::new(PgAuditLogRepo::new(pool.clone())),
        email: Arc::new(email),
        rate_limiter: Arc::new(RedisRateLimiter::new(redis.clone())),
        redis: redis.clone(),
        jwt_issuer: Arc::new(jwt_issuer),
    };

    let bind = format!("{}:{}", state.cfg.api_host, state.cfg.api_port);
    let state_for_factory = state.clone();
    // Build the OpenAPI spec once and clone the (cheap) `OpenApi` value per
    // worker. The `Option` matches the `build_app` signature so tests can
    // skip Swagger entirely by passing `None`.
    let openapi = ApiDoc::with_cookie_auth();
    HttpServer::new(move || build_app(state_for_factory.clone(), Some(openapi.clone())))
        .bind(&bind)?
        .run()
        .await?;

    if matches!(cfg.app_env, AppEnv::Production) {
        tracing::info!("api shutdown clean");
    }
    Ok(())
}
