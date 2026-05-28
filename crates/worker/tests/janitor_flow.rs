//! Integration test for the janitor sweep — seeds expired rows in each of the
//! four target tables and asserts they're deleted while younger ones survive.
//! Uses testcontainers like the digest-flow test; CI's backend-tests job runs
//! it on the Docker daemon there.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::future_not_send,
    clippy::similar_names,
    clippy::too_many_lines,
    reason = "test code: testcontainers + assertions may panic and aren't Send-bounded; the long set-up enumerates fixtures for four tables in one fn for readability"
)]

use std::time::Duration as StdDuration;

use chrono::{Duration, Utc};
use my_family_domain::JanitorRepo;
use my_family_persistence::{Database, PgJanitor};
use testcontainers::ContainerAsync;
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::postgres::Postgres;

async fn start_pg() -> (Database, ContainerAsync<Postgres>) {
    let pg = Postgres::default()
        .with_db_name("t")
        .with_user("t")
        .with_password("t")
        .start()
        .await
        .expect("start pg");
    let port = pg.get_host_port_ipv4(5432_u16).await.expect("pg port");
    let url = format!("postgres://t:t@127.0.0.1:{port}/t");
    let mut connected: Option<Database> = None;
    for _ in 0_u8..40_u8 {
        if let Ok(db) = Database::connect(&url, 4, StdDuration::from_secs(1), 30_000).await {
            connected = Some(db);
            break;
        }
        tokio::time::sleep(StdDuration::from_millis(250)).await;
    }
    let db = connected.expect("postgres never accepted connections");
    sqlx::migrate!("../../migrations").run(db.pool()).await.expect("migrate");
    (db, pg)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sweep_deletes_expired_rows_across_all_four_tables() {
    let (db, _pg) = start_pg().await;
    let pool = db.pool().clone();
    let janitor = PgJanitor::new(pool.clone());

    let now = Utc::now();
    let expired = now - Duration::hours(48); // well past a 24h grace
    let fresh = now + Duration::hours(1); // still active

    // Need a user to FK against for refresh_tokens + magic_link_tokens.
    let user_id: uuid::Uuid = sqlx::query_scalar!(
        "INSERT INTO users (email, locale) VALUES ('janitor@test.local', 'en') RETURNING id",
    )
    .fetch_one(&pool)
    .await
    .expect("seed user");

    // Need a family + invited_by FK to seed family_invites + owner_transfers.
    let family_id: uuid::Uuid = sqlx::query_scalar!(
        "INSERT INTO families (name, created_by) VALUES ('JanFam', $1) RETURNING id",
        user_id,
    )
    .fetch_one(&pool)
    .await
    .expect("seed family");

    // 1) magic_link_tokens — one expired, one fresh.
    sqlx::query!(
        "INSERT INTO magic_link_tokens (user_id, token_hash, purpose, email, expires_at) \
         VALUES ($1, $2, 'login', 'old@test.local', $3), \
                ($1, $4, 'login', 'new@test.local', $5)",
        user_id,
        b"old-magic" as &[u8],
        expired,
        b"new-magic" as &[u8],
        fresh,
    )
    .execute(&pool)
    .await
    .expect("seed magic links");

    // 2) refresh_tokens — absolute_expires_at past for one.
    sqlx::query!(
        "INSERT INTO refresh_tokens (user_id, token_hash, expires_at, absolute_expires_at) \
         VALUES ($1, $2, $3, $3), ($1, $4, $5, $5)",
        user_id,
        b"old-refresh" as &[u8],
        expired,
        b"new-refresh" as &[u8],
        fresh,
    )
    .execute(&pool)
    .await
    .expect("seed refresh tokens");

    // 3) family_invites — one expired, one fresh.
    sqlx::query!(
        "INSERT INTO family_invites (family_id, email, invited_role, invited_by, token_hash, expires_at) \
         VALUES ($1, 'old-inv@test.local', 'user', $2, $3, $4), \
                ($1, 'new-inv@test.local', 'user', $2, $5, $6)",
        family_id,
        user_id,
        b"old-invite" as &[u8],
        expired,
        b"new-invite" as &[u8],
        fresh,
    )
    .execute(&pool)
    .await
    .expect("seed invites");

    // 4) family_owner_transfers — one cancelled past grace (qualifies for sweep)
    //    + one fresh-pending. We use different families to avoid the
    //    "at most one pending transfer per family" partial-unique index.
    let family2_id: uuid::Uuid = sqlx::query_scalar!(
        "INSERT INTO families (name, created_by) VALUES ('JanFam2', $1) RETURNING id",
        user_id,
    )
    .fetch_one(&pool)
    .await
    .expect("seed family 2");
    sqlx::query!(
        "INSERT INTO family_owner_transfers \
            (family_id, from_user_id, to_user_id, from_token_hash, to_token_hash, expires_at, cancelled_at) \
         VALUES ($1, $2, $2, $3, $4, $5, $6), \
                ($7, $2, $2, $8, $9, $10, NULL)",
        family_id,
        user_id,
        b"old-from" as &[u8],
        b"old-to" as &[u8],
        expired,
        expired,
        family2_id,
        b"new-from" as &[u8],
        b"new-to" as &[u8],
        fresh,
    )
    .execute(&pool)
    .await
    .expect("seed owner transfers");

    // Sweep with zero grace — the "expired" rows are definitively past cutoff.
    let report = janitor.sweep_expired(now, Duration::seconds(0)).await.expect("sweep ok");
    assert_eq!(report.magic_links_deleted, 1, "one expired magic-link removed");
    assert_eq!(report.refresh_tokens_deleted, 1, "one expired refresh-token removed");
    assert_eq!(report.family_invites_deleted, 1, "one expired invite removed");
    assert_eq!(report.owner_transfers_deleted, 1, "one cancelled+expired transfer removed");
    assert_eq!(report.total(), 4);

    // The fresh rows survive — proves the sweep doesn't over-delete.
    let magic: i64 = sqlx::query_scalar!("SELECT COUNT(*) AS \"c!\" FROM magic_link_tokens")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(magic, 1);
    let refresh: i64 = sqlx::query_scalar!("SELECT COUNT(*) AS \"c!\" FROM refresh_tokens")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(refresh, 1);
    let invites: i64 = sqlx::query_scalar!("SELECT COUNT(*) AS \"c!\" FROM family_invites")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(invites, 1);
    let xfers: i64 = sqlx::query_scalar!("SELECT COUNT(*) AS \"c!\" FROM family_owner_transfers")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(xfers, 1);
}
