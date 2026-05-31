//! Deterministic dev/test seed orchestrator.
//!
//! Lives in a dedicated `crates/seeder/` crate — **not** in `my-fam-tree-api` —
//! so the production api image never ships a binary capable of mutating real
//! data via hardcoded UUIDs. The compose `seeder` service builds from
//! `.docker/seeder.Dockerfile` and is only invoked in dev (`rdt seed` /
//! `rdt reset`) and in CI test/coverage jobs.
//!
//! Module layout:
//! - [`ids`] — every hardcoded `Uuid` the seed inserts.
//! - [`persons`] — the 20-row `persons` table covering relationship edge
//!   cases (widow / divorce / same-sex / adoption / half-siblings /
//!   single parent / 4-generation depth).
//! - [`relationships`] — `parent_links` + `partnerships` with hardcoded
//!   ids so closed/widowed partnership rows stay idempotent across
//!   re-seeds.
//!
//! Every row is upserted with `ON CONFLICT … DO UPDATE`, so repeated
//! invocations are no-ops on row counts. The tree itself is acyclic by
//! construction; the seeder uses raw SQL repo-level inserts rather than the
//! HTTP handler's in-memory cycle check (which lives in
//! `routes::parent_links::create`, not in `ParentLinkRepo::insert`).

use std::sync::Arc;

use anyhow::Context;
use my_fam_tree_api::Config;
use my_fam_tree_api::services::auth_service::mint_magic_link_url;
use my_fam_tree_domain::{MagicLinkRepo, UserId};
use my_fam_tree_persistence::PgMagicLinkRepo;
use sqlx::PgPool;
use uuid::Uuid;

pub mod contacts;
pub mod ids;
pub mod persons;
pub mod relationships;

// Re-export the public UUIDs so call sites (and tests) keep the old import
// path `my_fam_tree_seeder::SEED_…_ID` working.
pub use ids::{
    SEED_ADMIN_USER_ID, SEED_ALICE_USER_ID, SEED_BOB_USER_ID, SEED_FAMILY_ID,
    SEED_PARTNERSHIP_FRIEDRICH_LOTTE_ID, SEED_PARTNERSHIP_KLAUS_ANNA_ID,
    SEED_PARTNERSHIP_KLAUS_BRIGITTE_ID, SEED_PARTNERSHIP_OTTO_HANNELORE_ID,
    SEED_PARTNERSHIP_SABINE_JULIA_ID, SEED_PARTNERSHIP_WERNER_GRETA_ID, SEED_PERSON_ANNA_ID,
    SEED_PERSON_BRIGITTE_ID, SEED_PERSON_COUNT, SEED_PERSON_EMMA_ID, SEED_PERSON_FELIX_ID,
    SEED_PERSON_FRIEDRICH_ID, SEED_PERSON_GRETA_ID, SEED_PERSON_HANNELORE_ID, SEED_PERSON_JULIA_ID,
    SEED_PERSON_KLAUS_ID, SEED_PERSON_LENA_ID, SEED_PERSON_LINA_ID, SEED_PERSON_LOTTE_ID,
    SEED_PERSON_MARKUS_ID, SEED_PERSON_MAX_ID, SEED_PERSON_MIA_ID, SEED_PERSON_NOAH_ID,
    SEED_PERSON_OTTO_ID, SEED_PERSON_SABINE_ID, SEED_PERSON_TOM_ID, SEED_PERSON_WERNER_ID,
};

/// Outcome of a `run_seed` invocation. Both counts always reflect what the
/// seed inserted/updated for this run (not "newly inserted"), since the
/// UPSERTs touch every row unconditionally.
#[derive(Debug)]
pub struct SeedReport {
    /// Number of users upserted (always 3 on a successful run).
    pub users_upserted: usize,
    /// Number of persons upserted (always `SEED_PERSON_COUNT` on success).
    pub persons_upserted: usize,
    /// One `(email, consume_url)` pair per seeded user, in admin/alice/bob order.
    pub magic_links: Vec<(String, String)>,
}

/// Run the deterministic seed against `pool`. Idempotent: a second
/// invocation against the same DB leaves row counts identical.
///
/// Mints one fresh single-use magic-link URL per seeded user (login
/// purpose) using the shared `mint_magic_link_url` helper, so the URLs
/// match the real `/auth/magic-link` flow byte-for-byte. The returned
/// URLs are intended for the seeder's stdout / docker logs — paste into
/// the browser to sign in as admin / alice / bob.
///
/// # Errors
/// Propagates any Postgres error from the upsert statements or magic-link mint.
pub async fn run_seed(pool: &PgPool, cfg: &Config) -> anyhow::Result<SeedReport> {
    seed_users(pool).await.context("seed users")?;
    seed_family(pool).await.context("seed family")?;
    seed_memberships(pool).await.context("seed family_memberships")?;
    persons::seed_persons(pool).await.context("seed persons")?;
    relationships::seed_parent_links(pool).await.context("seed parent_links")?;
    relationships::seed_partnerships(pool).await.context("seed partnerships")?;
    contacts::seed_contacts(pool).await.context("seed person_contacts")?;

    let magic_links_repo: Arc<dyn MagicLinkRepo> = Arc::new(PgMagicLinkRepo::new(pool.clone()));
    let magic_links = mint_magic_links(&magic_links_repo, cfg).await.context("mint magic links")?;

    Ok(SeedReport { users_upserted: 3, persons_upserted: SEED_PERSON_COUNT, magic_links })
}

async fn seed_users(pool: &PgPool) -> anyhow::Result<()> {
    // (id, email, display_name) tuples. CITEXT email column dedupes on lower-case.
    let rows: [(Uuid, &str, &str); 3] = [
        (SEED_ADMIN_USER_ID, "admin@example.com", "Admin"),
        (SEED_ALICE_USER_ID, "alice@example.com", "Alice"),
        (SEED_BOB_USER_ID, "bob@example.com", "Bob"),
    ];
    for (id, email, display_name) in rows {
        sqlx::query(
            "INSERT INTO users (id, email, display_name, locale, email_verified_at) \
             VALUES ($1, $2, $3, 'en', now()) \
             ON CONFLICT (id) DO UPDATE SET \
                 email = EXCLUDED.email, \
                 display_name = EXCLUDED.display_name, \
                 email_verified_at = COALESCE(users.email_verified_at, EXCLUDED.email_verified_at)",
        )
        .bind(id)
        .bind(email)
        .bind(display_name)
        .execute(pool)
        .await?;
    }
    Ok(())
}

async fn seed_family(pool: &PgPool) -> anyhow::Result<()> {
    sqlx::query(
        "INSERT INTO families (id, name, created_by) VALUES ($1, $2, $3) \
         ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name",
    )
    .bind(SEED_FAMILY_ID)
    .bind("Müller")
    .bind(SEED_ADMIN_USER_ID)
    .execute(pool)
    .await?;
    Ok(())
}

async fn seed_memberships(pool: &PgPool) -> anyhow::Result<()> {
    // (user_id, role).
    let rows: [(Uuid, &str); 3] =
        [(SEED_ADMIN_USER_ID, "owner"), (SEED_ALICE_USER_ID, "admin"), (SEED_BOB_USER_ID, "user")];
    for (user_id, role) in rows {
        sqlx::query(
            "INSERT INTO family_memberships (family_id, user_id, role) \
             VALUES ($1, $2, ($3::text)::family_role) \
             ON CONFLICT (family_id, user_id) DO UPDATE SET role = EXCLUDED.role",
        )
        .bind(SEED_FAMILY_ID)
        .bind(user_id)
        .bind(role)
        .execute(pool)
        .await?;
    }
    Ok(())
}

async fn mint_magic_links(
    repo: &Arc<dyn MagicLinkRepo>,
    cfg: &Config,
) -> anyhow::Result<Vec<(String, String)>> {
    let users: [(Uuid, &str); 3] = [
        (SEED_ADMIN_USER_ID, "admin@example.com"),
        (SEED_ALICE_USER_ID, "alice@example.com"),
        (SEED_BOB_USER_ID, "bob@example.com"),
    ];
    let mut out = Vec::with_capacity(users.len());
    for (uid, email) in users {
        let url = mint_magic_link_url(
            repo,
            UserId::from_uuid(uid),
            email,
            &cfg.web.public_url,
            cfg.magic_link.ttl_seconds,
        )
        .await?;
        out.push((email.to_string(), url));
    }
    Ok(out)
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::future_not_send,
    reason = "test code: container setup + assertion helpers may panic and aren't Send-bounded"
)]
mod tests {
    use std::time::Duration;

    use my_fam_tree_api::{AppEnv, LogFormat};
    use my_fam_tree_persistence::Database;
    use my_fam_tree_persistence::counts::Table;
    use testcontainers::ContainerAsync;
    use testcontainers::runners::AsyncRunner;
    use testcontainers_modules::postgres::Postgres;

    use super::*;

    // Expected row counts mirror the canonical seed shape. Update these
    // numbers whenever the persons / parent_links / partnerships tables in
    // the corresponding seed module grow or shrink.
    const EXPECTED_PARENT_LINKS: i64 = 46;
    const EXPECTED_PARTNERSHIPS: i64 = 20;
    const EXPECTED_CONTACTS: i64 = 9;

    struct Harness {
        pool: sqlx::PgPool,
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
        let pool = db.pool().clone();
        Harness { pool, cfg, _pg: pg }
    }

    // Counting rows goes through `persistence::counts::count_rows` so
    // raw SQL stays inside the persistence crate (architectural rule).
    async fn count(pool: &sqlx::PgPool, table: my_fam_tree_persistence::counts::Table) -> i64 {
        my_fam_tree_persistence::counts::count_rows(pool, table).await.expect("count")
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn seed_against_empty_db_inserts_expected_row_counts() {
        let h = start_pg().await;
        let report = run_seed(&h.pool, &h.cfg).await.expect("seed");

        assert_eq!(report.users_upserted, 3);
        assert_eq!(report.persons_upserted, SEED_PERSON_COUNT);
        assert_eq!(count(&h.pool, Table::Users).await, 3);
        assert_eq!(count(&h.pool, Table::Families).await, 1);
        assert_eq!(count(&h.pool, Table::FamilyMemberships).await, 3);
        assert_eq!(count(&h.pool, Table::Persons).await, i64::try_from(SEED_PERSON_COUNT).unwrap());
        assert_eq!(count(&h.pool, Table::ParentLinks).await, EXPECTED_PARENT_LINKS);
        assert_eq!(count(&h.pool, Table::Partnerships).await, EXPECTED_PARTNERSHIPS);
        assert_eq!(count(&h.pool, Table::PersonContacts).await, EXPECTED_CONTACTS);

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
        let _ = run_seed(&h.pool, &h.cfg).await.expect("first seed");
        let r2 = run_seed(&h.pool, &h.cfg).await.expect("second seed");

        // Row counts unchanged after a second invocation.
        assert_eq!(count(&h.pool, Table::Users).await, 3);
        assert_eq!(count(&h.pool, Table::Families).await, 1);
        assert_eq!(count(&h.pool, Table::FamilyMemberships).await, 3);
        assert_eq!(count(&h.pool, Table::Persons).await, i64::try_from(SEED_PERSON_COUNT).unwrap());
        assert_eq!(count(&h.pool, Table::ParentLinks).await, EXPECTED_PARENT_LINKS);
        assert_eq!(count(&h.pool, Table::Partnerships).await, EXPECTED_PARTNERSHIPS);
        assert_eq!(count(&h.pool, Table::PersonContacts).await, EXPECTED_CONTACTS);

        // The second invocation still mints fresh magic links (one per user).
        // Magic-link tokens are append-only — count rises across calls.
        assert_eq!(r2.magic_links.len(), 3);
        assert_eq!(count(&h.pool, Table::MagicLinkTokens).await, 6);
    }
}
