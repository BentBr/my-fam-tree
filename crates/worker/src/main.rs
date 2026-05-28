//! Reminder worker: a leader-locked loop that schedules daily digests at each
//! user's local 06:00, plus a pool of dispatcher tasks that send them.

use std::sync::Arc;
use std::time::Duration;

use anyhow::Context;
use my_family_cache::{RedisPool, RedisReminderQueue};
use my_family_email::SmtpSender;
use my_family_persistence::{
    Database, PgEmailOutboxRepo, PgFamilyMembershipRepo, PgJanitor, PgPartnershipRepo,
    PgPersonFavouriteRepo, PgPersonRepo, PgReminderDigestRepo, PgReminderPrefsRepo, PgUserRepo,
};
use my_family_worker::clock::Clock;
#[cfg(feature = "test-fixtures")]
use my_family_worker::clock::FixedClock;
#[cfg(not(feature = "test-fixtures"))]
use my_family_worker::clock::SystemClock;
use my_family_worker::state::WorkerState;
#[cfg(feature = "test-fixtures")]
use my_family_worker::test_clock_http;
use my_family_worker::{config, dispatcher, janitor, leader, outbox, ticker};
use tokio::time::sleep;
use tracing_subscriber::prelude::*;

/// Number of concurrent dispatcher tasks draining the digest queue.
const DISPATCHER_POOL: usize = 4;

/// The leader-locked inner loop: digest ticker + janitor sweeps. Extracted
/// out of `main` so `main` stays focused on wiring + collaborator setup and
/// this hot loop is testable / readable on its own.
async fn run_leader_loop(
    worker: WorkerState,
    leader: leader::Leader,
    refresh: Duration,
    tick: Duration,
    janitor_tick: Duration,
) -> ! {
    loop {
        leader.acquire_blocking().await;
        tracing::info!("acquired leader lock");
        let mut last_tick =
            std::time::Instant::now().checked_sub(tick).unwrap_or_else(std::time::Instant::now);
        let mut last_janitor = std::time::Instant::now()
            .checked_sub(janitor_tick)
            .unwrap_or_else(std::time::Instant::now);
        loop {
            if !leader.refresh().await {
                tracing::warn!("lost leader lock; will re-acquire");
                break;
            }
            if last_tick.elapsed() >= tick {
                match ticker::run_tick(&worker).await {
                    Ok(n) if n > 0 => tracing::info!(scheduled = n, "tick scheduled digests"),
                    Ok(_) => {}
                    Err(e) => tracing::error!(?e, "tick error"),
                }
                last_tick = std::time::Instant::now();
            }
            if last_janitor.elapsed() >= janitor_tick {
                janitor::run_sweep(&worker).await;
                last_janitor = std::time::Instant::now();
            }
            sleep(refresh).await;
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    if std::env::var("APP_ENV").as_deref() == Ok("development") {
        let _ = dotenvy::dotenv();
    }
    let cfg = config::Config::from_env().context("load worker config")?;

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
    .await
    .context("connect postgres pool")?;
    db.ping().await.context("ping postgres")?;

    let redis = RedisPool::build(&cfg.redis_url, cfg.redis_max_connections, &cfg.redis_key_prefix)
        .context("build redis pool")?;
    redis.ping().await.context("ping redis")?;

    let email = Arc::new(
        SmtpSender::from_dsn(
            &cfg.email_dsn,
            &cfg.email_from_name,
            &cfg.email_from_address,
            cfg.email_reply_to.as_deref(),
            cfg.email_timeout_seconds,
        )
        .context("build SMTP sender")?,
    ) as Arc<dyn my_family_email::EmailSender>;

    #[cfg(feature = "test-fixtures")]
    let fixed = Arc::new(FixedClock::new(chrono::Utc::now()));
    #[cfg(feature = "test-fixtures")]
    let clock: Arc<dyn Clock> = fixed.clone();
    #[cfg(not(feature = "test-fixtures"))]
    let clock: Arc<dyn Clock> = Arc::new(SystemClock);

    let pool = db.pool().clone();
    let worker = WorkerState {
        clock,
        users: Arc::new(PgUserRepo::new(pool.clone())),
        memberships: Arc::new(PgFamilyMembershipRepo::new(pool.clone())),
        persons: Arc::new(PgPersonRepo::new(pool.clone())),
        partnerships: Arc::new(PgPartnershipRepo::new(pool.clone())),
        favourites: Arc::new(PgPersonFavouriteRepo::new(pool.clone())),
        prefs: Arc::new(PgReminderPrefsRepo::new(pool.clone())),
        digests: Arc::new(PgReminderDigestRepo::new(pool.clone())),
        queue: Arc::new(RedisReminderQueue::new(redis.clone())),
        email,
        janitor: Arc::new(PgJanitor::new(pool.clone())),
        outbox: Arc::new(PgEmailOutboxRepo::new(pool.clone())),
        web_public_url: cfg.web_public_url.clone(),
        max_retries: cfg.worker_max_retries,
        retry_min_seconds: cfg.worker_retry_backoff_min_seconds,
        retry_max_seconds: cfg.worker_retry_backoff_max_seconds,
        janitor_grace_seconds: cfg.worker_janitor_grace_seconds,
    };

    for _ in 0..DISPATCHER_POOL {
        let s = worker.clone();
        tokio::spawn(async move { dispatcher::run_dispatcher(s).await });
    }

    // Outbox pollers — lock-free (FOR UPDATE SKIP LOCKED in claim_next_due),
    // so they can drain a backlog in parallel across pool size + replicas.
    let outbox_poll = Duration::from_secs(cfg.worker_outbox_poll_seconds);
    for _ in 0..cfg.worker_outbox_pool_size {
        let s = worker.clone();
        tokio::spawn(async move { outbox::run_poller(s, outbox_poll).await });
    }

    #[cfg(feature = "test-fixtures")]
    {
        // actix-web's HttpServer future is not Send (Rc internals), so it can't
        // ride the main multi-threaded tokio runtime via tokio::spawn. Run it on
        // a dedicated thread with its own actix system. test-fixtures only.
        let s = worker.clone();
        let bind = cfg.worker_metrics_bind.clone();
        let fixed_handle = fixed.clone();
        std::thread::spawn(move || {
            actix_web::rt::System::new().block_on(test_clock_http::serve(s, fixed_handle, bind));
        });
    }

    let leader = leader::Leader::new(redis, Duration::from_secs(cfg.worker_leader_lease_seconds));
    let refresh = Duration::from_secs(cfg.worker_leader_refresh_seconds);
    let tick = Duration::from_secs(cfg.worker_tick_interval_seconds);
    let janitor_tick = Duration::from_secs(cfg.worker_janitor_interval_seconds);
    tracing::info!(
        app_env = %cfg.app_env,
        janitor_interval_s = cfg.worker_janitor_interval_seconds,
        janitor_grace_s = cfg.worker_janitor_grace_seconds,
        "worker started",
    );
    run_leader_loop(worker, leader, refresh, tick, janitor_tick).await
}
