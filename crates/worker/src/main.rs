//! Reminder worker: a leader-locked loop that schedules daily digests at each
//! user's local 06:00, plus a pool of dispatcher tasks that send them.

use std::sync::Arc;
use std::time::Duration;

use anyhow::Context;
use my_fam_tree_cache::{RedisPool, RedisReminderQueue};
use my_fam_tree_config::WorkerConfig;
use my_fam_tree_email::SmtpSender;
use my_fam_tree_persistence::{
    Database, PgEmailOutboxRepo, PgFamilyMembershipRepo, PgJanitor, PgPartnershipRepo,
    PgPersonFavouriteRepo, PgPersonRepo, PgReminderDigestRepo, PgReminderPrefsRepo, PgUserRepo,
};
use my_fam_tree_worker::clock::Clock;
#[cfg(feature = "test-fixtures")]
use my_fam_tree_worker::clock::OffsetClock;
#[cfg(not(feature = "test-fixtures"))]
use my_fam_tree_worker::clock::SystemClock;
use my_fam_tree_worker::health::Heartbeat;
use my_fam_tree_worker::state::WorkerState;
use my_fam_tree_worker::{dispatcher, janitor, leader, ops_http, outbox, ticker};
use tokio::time::sleep;
use tracing_subscriber::prelude::*;

/// Number of concurrent dispatcher tasks draining the digest queue.
const DISPATCHER_POOL: usize = 4;

/// The leader-locked inner loop: digest ticker + janitor sweeps. Extracted
/// out of `main` so `main` stays focused on wiring + collaborator setup and
/// this hot loop is testable / readable on its own.
///
/// `heartbeat.beat()` fires on every iteration before the sleep so the
/// `/health` endpoint can detect "main loop is wedged" (e.g. a blocking
/// DB call) — a TCP-accept probe alone would still answer 200 in that
/// case. Heartbeats happen INSIDE the loop, so a worker that hasn't
/// acquired the leader lock yet (cold start) is correctly reported as
/// "not yet healthy" until `acquire_blocking` returns and the first tick
/// runs.
async fn run_leader_loop(
    worker: WorkerState,
    leader: leader::Leader,
    refresh: Duration,
    tick: Duration,
    janitor_tick: Duration,
    heartbeat: Arc<Heartbeat>,
) -> ! {
    loop {
        leader.acquire_blocking().await;
        tracing::info!("acquired leader lock");
        // First beat right after acquiring the lock — the loop is now
        // doing useful work, and the next iteration's `sleep(refresh)`
        // shouldn't make us look stale before it's even ticked once.
        heartbeat.beat();
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
            heartbeat.beat();
            sleep(refresh).await;
        }
    }
}

// `main` is mostly linear wiring of collaborator deps + the leader-loop
// launch. Splitting it into a sub-fn just moves the cfg-gated branches
// to a new boundary without making either side simpler, so we accept
// the length here.
#[allow(clippy::too_many_lines, reason = "linear wiring sequence — splitting hides the order")]
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    if std::env::var("APP_ENV").as_deref() == Ok("development") {
        let _ = dotenvy::dotenv();
    }
    let cfg = WorkerConfig::from_env().context("load worker config")?;

    let filter = tracing_subscriber::EnvFilter::try_new(&cfg.log.level)
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));
    let registry = tracing_subscriber::registry().with(filter);
    if matches!(cfg.log.format, my_fam_tree_config::LogFormat::Json) {
        registry.with(tracing_subscriber::fmt::layer().json()).init();
    } else {
        registry.with(tracing_subscriber::fmt::layer().pretty()).init();
    }

    let db = Database::connect(
        &cfg.database.url,
        cfg.database.max_connections,
        Duration::from_secs(cfg.database.acquire_timeout_seconds),
        cfg.database.statement_timeout_ms,
    )
    .await
    .context("connect postgres pool")?;
    db.ping().await.context("ping postgres")?;

    let redis = RedisPool::build(&cfg.redis.url, cfg.redis.max_connections, &cfg.redis.key_prefix)
        .context("build redis pool")?;
    redis.ping().await.context("ping redis")?;

    let email = Arc::new(
        SmtpSender::from_dsn(
            &cfg.email.dsn,
            &cfg.email.from_name,
            &cfg.email.from_address,
            cfg.email.reply_to.as_deref(),
            cfg.email.timeout_seconds,
        )
        .context("build SMTP sender")?,
    ) as Arc<dyn my_fam_tree_email::EmailSender>;

    #[cfg(feature = "test-fixtures")]
    let fixed = Arc::new(OffsetClock::default());
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
        web_public_url: cfg.web.public_url.clone(),
        max_retries: cfg.worker.max_retries,
        retry_min_seconds: cfg.worker.retry_backoff_min_seconds,
        retry_max_seconds: cfg.worker.retry_backoff_max_seconds,
        janitor_grace_seconds: cfg.janitor.grace_seconds,
    };

    for _ in 0..DISPATCHER_POOL {
        let s = worker.clone();
        tokio::spawn(async move { dispatcher::run_dispatcher(s).await });
    }

    // Outbox pollers — lock-free (FOR UPDATE SKIP LOCKED in claim_next_due),
    // so they can drain a backlog in parallel across pool size + replicas.
    let outbox_poll = Duration::from_secs(cfg.outbox.poll_seconds);
    for _ in 0..cfg.outbox.pool_size {
        let s = worker.clone();
        tokio::spawn(async move { outbox::run_poller(s, outbox_poll).await });
    }

    // Heartbeat shared between the leader loop (writer) and the /health
    // listener (reader). Created here so we can hand the same Arc to both.
    let heartbeat = Heartbeat::new();

    // Staleness threshold: 2× the leader-refresh interval, plus a 5 s
    // jitter margin so a one-cycle blip doesn't flip the probe to red.
    // The refresh interval is the loop's natural cadence; anything past
    // two of those is almost certainly a wedged loop, not just slow.
    let refresh_ms = i64::try_from(cfg.worker.leader_refresh_seconds * 1000).unwrap_or(i64::MAX);
    let stale_after_ms = refresh_ms.saturating_mul(2).saturating_add(5_000);

    // actix-web's HttpServer future is not Send (Rc internals), so it
    // can't ride the main multithreaded tokio runtime via tokio::spawn.
    // Run it on a dedicated thread with its own actix system — same
    // trick the old test-fixtures listener used; now hosts /health
    // unconditionally + /__test/advance-clock when the feature is on.
    let bind = cfg.worker.metrics_bind.clone();
    let hb_for_http = heartbeat.clone();
    #[cfg(not(feature = "test-fixtures"))]
    std::thread::spawn(move || {
        actix_web::rt::System::new().block_on(ops_http::serve(hb_for_http, stale_after_ms, bind));
    });
    #[cfg(feature = "test-fixtures")]
    {
        let s = worker.clone();
        let fixed_handle = fixed.clone();
        std::thread::spawn(move || {
            actix_web::rt::System::new().block_on(ops_http::serve_with_test_fixtures(
                hb_for_http,
                stale_after_ms,
                bind,
                s,
                fixed_handle,
            ));
        });
    }

    let leader = leader::Leader::new(redis, Duration::from_secs(cfg.worker.leader_lease_seconds));
    let refresh = Duration::from_secs(cfg.worker.leader_refresh_seconds);
    let tick = Duration::from_secs(cfg.worker.tick_interval_seconds);
    let janitor_tick = Duration::from_secs(cfg.janitor.interval_seconds);
    tracing::info!(
        app_env = %cfg.app_env,
        janitor_interval_s = cfg.janitor.interval_seconds,
        janitor_grace_s = cfg.janitor.grace_seconds,
        health_stale_after_ms = stale_after_ms,
        "worker started",
    );
    run_leader_loop(worker, leader, refresh, tick, janitor_tick, heartbeat).await
}
