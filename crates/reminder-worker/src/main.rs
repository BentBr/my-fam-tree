use std::time::Duration;

use my_family_cache::RedisPool;
use my_family_persistence::Database;
use tracing_subscriber::prelude::*;

#[derive(Debug, serde::Deserialize)]
struct WorkerConfig {
    database_url: String,
    database_max_connections: u32,
    database_acquire_timeout_seconds: u64,
    database_statement_timeout_ms: u32,
    redis_url: String,
    redis_max_connections: usize,
    redis_key_prefix: String,
    worker_tick_interval_seconds: u64,
    log_format: String,
    rust_log: String,
    app_env: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    if std::env::var("APP_ENV").as_deref() == Ok("development") {
        let _ = dotenvy::dotenv();
    }

    let cfg: WorkerConfig = envy::from_env()?;

    let filter = tracing_subscriber::EnvFilter::try_new(&cfg.rust_log)
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));
    let registry = tracing_subscriber::registry().with(filter);
    if cfg.log_format == "json" {
        registry.with(tracing_subscriber::fmt::layer().json()).init();
    } else {
        registry.with(tracing_subscriber::fmt::layer().pretty()).init();
    }

    let db = Database::connect(
        &cfg.database_url,
        cfg.database_max_connections,
        Duration::from_secs(cfg.database_acquire_timeout_seconds),
        cfg.database_statement_timeout_ms,
    )
    .await?;
    db.ping().await?;
    let redis = RedisPool::build(&cfg.redis_url, cfg.redis_max_connections, &cfg.redis_key_prefix)?;
    redis.ping().await?;

    tracing::info!(
        app_env = %cfg.app_env,
        tick_interval_seconds = cfg.worker_tick_interval_seconds,
        "reminder-worker started; tick loop implemented in phase 4",
    );

    let tick = Duration::from_secs(cfg.worker_tick_interval_seconds);
    loop {
        tracing::debug!("tick (no-op in phase 0)");
        tokio::time::sleep(tick).await;
    }
}
