// This binary is a CLI; --status, --check, and --dry-run all print
// human-readable migration state to stdout/stderr by design.
#![allow(clippy::print_stdout, clippy::print_stderr)]

use std::time::Duration;

use clap::Parser;
use sqlx::migrate::{Migrate, Migrator};
use sqlx::postgres::PgPoolOptions;

#[derive(Debug, Parser)]
#[command(name = "run_migrations", version, about = "Apply, inspect, or check SQLx migrations")]
struct Args {
    /// Show which migrations are applied vs pending and exit.
    #[arg(long)]
    status: bool,
    /// Exit non-zero if any migrations are pending (no changes applied).
    #[arg(long)]
    check: bool,
    /// Print which migrations *would* be applied; do not run them.
    #[arg(long)]
    dry_run: bool,
    /// Apply up to (and including) this version, then stop.
    #[arg(long)]
    target: Option<i64>,
    /// `DATABASE_URL` override.
    #[arg(long, env = "DATABASE_URL")]
    database_url: String,
}

static MIGRATOR: Migrator = sqlx::migrate!("../../migrations");

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    let args = Args::parse();

    let pool = PgPoolOptions::new()
        .max_connections(2)
        .acquire_timeout(Duration::from_secs(5))
        .connect(&args.database_url)
        .await?;

    let mut conn = pool.acquire().await?;
    conn.ensure_migrations_table().await?;

    let applied: std::collections::HashMap<i64, sqlx::migrate::AppliedMigration> =
        conn.list_applied_migrations().await?.into_iter().map(|m| (m.version, m)).collect();

    let mut total = 0usize;
    let mut pending = 0usize;

    for m in MIGRATOR.iter() {
        total += 1;
        let is_applied = applied.contains_key(&m.version);
        if !is_applied {
            pending += 1;
        }
        if args.status {
            let state = if is_applied { "APPLIED" } else { "PENDING" };
            println!("{state:8} {:>4}  {}", m.version, m.description);
        }
    }

    if args.status {
        println!();
        println!("{total} total, {pending} pending.");
        return Ok(());
    }

    if args.check {
        if pending > 0 {
            eprintln!("{pending} pending migration(s).");
            std::process::exit(2);
        }
        println!("Up to date ({total} applied).");
        return Ok(());
    }

    if args.dry_run {
        for m in MIGRATOR.iter() {
            if !applied.contains_key(&m.version) {
                if matches!(args.target, Some(t) if m.version > t) {
                    break;
                }
                println!("WOULD APPLY {:>4} {}", m.version, m.description);
            }
        }
        return Ok(());
    }

    drop(conn);

    if let Some(target) = args.target {
        let mut conn = pool.acquire().await?;
        for m in MIGRATOR.iter() {
            if m.version > target {
                break;
            }
            if !applied.contains_key(&m.version) {
                conn.apply(m).await?;
                tracing::info!(version = m.version, description = %m.description, "applied");
            }
        }
    } else {
        MIGRATOR.run(&pool).await?;
    }

    tracing::info!("migrations complete");
    Ok(())
}
