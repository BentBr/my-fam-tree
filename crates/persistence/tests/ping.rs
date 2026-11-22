//! Skipped when `DATABASE_URL` is unset. Run via `./scripts/cargo-in-network.sh
//! test -p my-fam-tree-persistence --test ping` so the test sees the compose
//! `postgres` service at its network hostname.

#![expect(
    clippy::disallowed_methods,
    reason = "integration test opts in via DATABASE_URL from cargo-in-network.sh; there is no app config to read it through"
)]

use std::time::Duration;

use my_fam_tree_persistence::Database;

#[tokio::test]
async fn pings_postgres() {
    let Ok(url) = std::env::var("DATABASE_URL") else {
        eprintln!("DATABASE_URL not set; skipping");
        return;
    };
    let db = Database::connect(&url, 4, Duration::from_secs(5), 30_000).await.expect("connect");
    db.ping().await.expect("ping");
}
