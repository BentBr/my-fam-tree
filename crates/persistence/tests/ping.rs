//! Skipped when `DATABASE_URL` is unset. Run via `./scripts/cargo-in-network.sh
//! test -p my-family-persistence --test ping` so the test sees the compose
//! `postgres` service at its network hostname.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::print_stderr)]

use std::time::Duration;

use my_family_persistence::Database;

#[tokio::test]
async fn pings_postgres() {
    let Ok(url) = std::env::var("DATABASE_URL") else {
        eprintln!("DATABASE_URL not set; skipping");
        return;
    };
    let db = Database::connect(&url, 4, Duration::from_secs(5), 30_000).await.expect("connect");
    db.ping().await.expect("ping");
}
