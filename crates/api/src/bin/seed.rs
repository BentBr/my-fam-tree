//! Thin entry point for the `seed` binary. Loads config, opens a Postgres pool,
//! runs the deterministic seed, and prints the minted magic-link URLs to stdout
//! (grep-friendly: `MAGIC_LINK <email> <url>` per user).

use std::time::Duration;

use anyhow::Context;
use my_family_api::{Config, init_tracing, seed};
use my_family_persistence::Database;

#[allow(
    clippy::print_stdout,
    reason = "this binary's job is to surface seeded magic-link URLs on stdout"
)]
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    if std::env::var("APP_ENV").as_deref() == Ok("development") {
        // Optional .env, same handling as the api binary — ignore missing/parse
        // errors here because tracing isn't initialized yet and the workspace
        // forbids println/eprintln outside the seeder's main itself.
        let _ = dotenvy::dotenv();
    }

    let cfg = Config::load_from_env().context("load config from environment")?;
    init_tracing(cfg.log_format, &cfg.rust_log);

    let db = Database::connect(
        &cfg.database_url,
        cfg.database_max_connections,
        Duration::from_secs(cfg.database_acquire_timeout_seconds),
        cfg.database_statement_timeout_ms,
    )
    .await
    .context("connect postgres pool")?;

    let report = seed::run_seed(db.pool(), &cfg).await.context("run seed")?;

    println!("seeded {} users, {} persons", report.users_upserted, report.persons_upserted);
    for (email, url) in &report.magic_links {
        println!("MAGIC_LINK {email} {url}");
    }
    Ok(())
}
