//! The deterministic seed against a throwaway Postgres: expected row counts
//! and idempotency. Lives in tests/ (not src/) because it runs migrations
//! with sqlx directly — src/ stays SQL-free (arch-tests).

#![expect(
    clippy::expect_used,
    clippy::cast_possible_wrap,
    reason = "test code: setup failures abort the test with context; tiny fixture counts cannot wrap"
)]

use std::time::Duration;

use my_fam_tree_api::{AppEnv, Config, LogFormat};
use my_fam_tree_persistence::Database;
use my_fam_tree_persistence::counts::Table;
use my_fam_tree_persistence::seed::SeedDb;
use my_fam_tree_seeder::{SEED_PERSON_COUNT, lang, run_seed, vellmar};
use testcontainers::ContainerAsync;
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::postgres::Postgres;

// Expected row counts mirror the canonical seed shape across ALL
// three seeded families (Müller + Vellmar + Lang). Update these
// numbers whenever any seed module's tables grow or shrink.
const EXPECTED_PERSONS: i64 =
    SEED_PERSON_COUNT as i64 + vellmar::PERSON_COUNT as i64 + lang::PERSON_COUNT as i64;
const EXPECTED_PARENT_LINKS: i64 = 44 + vellmar::PARENT_LINK_COUNT + lang::PARENT_LINK_COUNT;
const EXPECTED_PARTNERSHIPS: i64 = 18 + vellmar::PARTNERSHIP_COUNT;
const EXPECTED_CONTACTS: i64 = 9;

struct Harness {
    db: Database,
    cfg: Config,
    _pg: ContainerAsync<Postgres>,
}

async fn start_pg() -> Harness {
    let pg = Postgres::default()
        .with_db_name("t")
        .with_user("t")
        .with_password("t")
        .start()
        .await
        .expect("start pg");
    let port = pg.get_host_port_ipv4(5432_u16).await.expect("pg port");
    let url = format!("postgres://t:t@127.0.0.1:{port}/t");

    // Connection can lag a few ms behind the readiness log; retry briefly.
    let mut connected: Option<Database> = None;
    for _ in 0_u8..40_u8 {
        if let Ok(db) = Database::connect(&url, 2, Duration::from_secs(1), 30_000).await {
            connected = Some(db);
            break;
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
    let db = connected.expect("postgres never accepted connections");
    sqlx::migrate!("../../migrations").run(db.pool()).await.expect("migrate");

    let cfg = Config {
        app_env: AppEnv::Development,
        log: my_fam_tree_config::LogConfig { level: "info".into(), format: LogFormat::Pretty },
        api: my_fam_tree_config::ApiBindConfig {
            host: "0.0.0.0".into(),
            port: 8080,
            public_url: "http://localhost:8080".into(),
            cors_allowed_origins: "http://localhost:5173".into(),
            enable_docs: false,
            metrics_bind: "0.0.0.0:9090".into(),
        },
        web: my_fam_tree_config::WebConfig { public_url: "http://my-fam-tree.docker".into() },
        database: my_fam_tree_config::DatabaseConfig {
            url: url.clone(),
            max_connections: 4,
            acquire_timeout_seconds: 5,
            statement_timeout_ms: 30_000,
        },
        redis: my_fam_tree_config::RedisConfig {
            url: "redis://localhost".into(),
            max_connections: 4,
            key_prefix: "t:".into(),
        },
        jwt: my_fam_tree_config::JwtConfig {
            private_key: "x".into(),
            private_key_id: "t".into(),
            public_keys: "[]".into(),
            issuer: "iss".into(),
            audience: "aud".into(),
            access_ttl_seconds: 900,
            refresh_ttl_seconds: 86_400,
            refresh_absolute_ttl_seconds: 604_800,
        },
        cookie: my_fam_tree_config::CookieConfig {
            domain: String::new(),
            secure: false,
            samesite_access: "Lax".into(),
            samesite_refresh: "Strict".into(),
        },
        magic_link: my_fam_tree_config::MagicLinkConfig {
            ttl_seconds: 900,
            invite_ttl_seconds: 1_209_600,
            rate_per_email_per_hour: 10,
            rate_per_ip_per_hour: 100,
        },
        email: my_fam_tree_config::EmailConfig {
            dsn: "smtp://localhost:1025".into(),
            from_name: "t".into(),
            from_address: "no-reply@t".into(),
            reply_to: None,
            timeout_seconds: 10,
        },
        storage: my_fam_tree_config::StorageConfig {
            driver: my_fam_tree_config::storage::StorageDriver::Local,
            bucket: "t".into(),
            region: "us-east-1".into(),
            endpoint_url: None,
            access_key_id: String::new(),
            secret_access_key: String::new(),
            force_path_style: false,
        },
    };
    Harness { db, cfg, _pg: pg }
}

// Counting rows goes through `persistence::counts::count_rows` so
// raw SQL stays inside the persistence crate (architectural rule).
async fn count(pool: &sqlx::PgPool, table: Table) -> i64 {
    my_fam_tree_persistence::counts::count_rows(pool, table).await.expect("count")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn seed_against_empty_db_inserts_expected_row_counts() {
    let h = start_pg().await;
    let report = run_seed(&SeedDb::new(&h.db), &h.cfg).await.expect("seed");

    assert_eq!(report.users_upserted, 3);
    assert_eq!(report.persons_upserted, usize::try_from(EXPECTED_PERSONS).unwrap());
    assert_eq!(count(h.db.pool(), Table::Users).await, 3);
    // Three seeded families (Müller + Vellmar + Lang); every user is
    // a member of every family → 3 × 3 memberships.
    assert_eq!(count(h.db.pool(), Table::Families).await, 3);
    assert_eq!(count(h.db.pool(), Table::FamilyMemberships).await, 9);
    assert_eq!(count(h.db.pool(), Table::Persons).await, EXPECTED_PERSONS);
    assert_eq!(count(h.db.pool(), Table::ParentLinks).await, EXPECTED_PARENT_LINKS);
    assert_eq!(count(h.db.pool(), Table::Partnerships).await, EXPECTED_PARTNERSHIPS);
    assert_eq!(count(h.db.pool(), Table::PersonContacts).await, EXPECTED_CONTACTS);

    assert_eq!(report.magic_links.len(), 3);
    for (email, url) in &report.magic_links {
        assert!(!email.is_empty(), "magic-link email must be non-empty");
        assert!(url.contains("/auth/consume?token="), "url must point at consume: {url}");
        assert!(url.starts_with("http://my-fam-tree.docker"), "url uses web_public_url: {url}");
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn seed_is_idempotent_on_row_counts() {
    let h = start_pg().await;
    drop(run_seed(&SeedDb::new(&h.db), &h.cfg).await.expect("first seed"));
    let r2 = run_seed(&SeedDb::new(&h.db), &h.cfg).await.expect("second seed");

    // Row counts unchanged after a second invocation.
    assert_eq!(count(h.db.pool(), Table::Users).await, 3);
    assert_eq!(count(h.db.pool(), Table::Families).await, 3);
    assert_eq!(count(h.db.pool(), Table::FamilyMemberships).await, 9);
    assert_eq!(count(h.db.pool(), Table::Persons).await, EXPECTED_PERSONS);
    assert_eq!(count(h.db.pool(), Table::ParentLinks).await, EXPECTED_PARENT_LINKS);
    assert_eq!(count(h.db.pool(), Table::Partnerships).await, EXPECTED_PARTNERSHIPS);
    assert_eq!(count(h.db.pool(), Table::PersonContacts).await, EXPECTED_CONTACTS);

    // The second invocation still mints fresh magic links (one per user).
    // Magic-link tokens are append-only — count rises across calls.
    assert_eq!(r2.magic_links.len(), 3);
    assert_eq!(count(h.db.pool(), Table::MagicLinkTokens).await, 6);
}
