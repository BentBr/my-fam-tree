//! Deterministic dev/test seed: 3 users, 1 family, 3 memberships, 8 persons,
//! 8 `parent_links`, 3 partnerships, and one minted magic-link URL per user.
//!
//! Every row is keyed on hardcoded `Uuid`s and inserted with `ON CONFLICT … DO
//! UPDATE`, so repeated invocations are no-ops on row counts. The tree itself
//! (G1: Otto/Hannelore/Werner/Greta · G2: Klaus/Anna · G3: Lina/Max) is acyclic
//! by construction, so we use raw SQL repo-level inserts rather than the HTTP
//! handler's in-memory cycle check (which lives in `routes::parent_links::create`,
//! not in `ParentLinkRepo::insert`). Seed data is hand-curated; the check is
//! redundant here.
//!
//! UPSERT strategy is **raw SQL** rather than repo methods: the repos don't
//! expose `upsert_with_id` everywhere we need it, and the seed module is the
//! one place where raw INSERT…ON CONFLICT is simpler than threading a new
//! method through every repo trait. Domain types are not used directly — we
//! talk to Postgres in `(uuid, text, …)` columns and rely on the migration to
//! shape the schema.

use std::sync::Arc;

use anyhow::Context;
use my_family_domain::{MagicLinkRepo, UserId};
use my_family_persistence::PgMagicLinkRepo;
use sqlx::PgPool;
use uuid::Uuid;

use crate::Config;
use crate::services::auth_service::mint_magic_link_url;

// ---------------------------------------------------------------------------
// Hardcoded UUIDs. Structured hex blocks make the seeded rows immediately
// recognisable in `psql` inspection (users 0x…0001_…, family 0x…0002_…,
// persons 0x…0003_…).
// ---------------------------------------------------------------------------

/// Seeded admin user (owner of the seeded family).
pub const SEED_ADMIN_USER_ID: Uuid = Uuid::from_u128(0x0000_0001_0000_0000_0000_0000_0000_0001);
/// Seeded user "Alice" (admin role).
pub const SEED_ALICE_USER_ID: Uuid = Uuid::from_u128(0x0000_0001_0000_0000_0000_0000_0000_0002);
/// Seeded user "Bob" (user role).
pub const SEED_BOB_USER_ID: Uuid = Uuid::from_u128(0x0000_0001_0000_0000_0000_0000_0000_0003);

/// The single seeded family.
pub const SEED_FAMILY_ID: Uuid = Uuid::from_u128(0x0000_0002_0000_0000_0000_0000_0000_0001);

// G1.
/// Seeded person "Otto" (G1).
pub const SEED_PERSON_OTTO_ID: Uuid = Uuid::from_u128(0x0000_0003_0000_0000_0000_0000_0000_0001);
/// Seeded person "Hannelore" (G1).
pub const SEED_PERSON_HANNELORE_ID: Uuid =
    Uuid::from_u128(0x0000_0003_0000_0000_0000_0000_0000_0002);
/// Seeded person "Werner" (G1).
pub const SEED_PERSON_WERNER_ID: Uuid = Uuid::from_u128(0x0000_0003_0000_0000_0000_0000_0000_0003);
/// Seeded person "Greta" (G1).
pub const SEED_PERSON_GRETA_ID: Uuid = Uuid::from_u128(0x0000_0003_0000_0000_0000_0000_0000_0004);
// G2.
/// Seeded person "Klaus" (G2) — linked to admin user.
pub const SEED_PERSON_KLAUS_ID: Uuid = Uuid::from_u128(0x0000_0003_0000_0000_0000_0000_0000_0005);
/// Seeded person "Anna" (G2) — linked to alice user.
pub const SEED_PERSON_ANNA_ID: Uuid = Uuid::from_u128(0x0000_0003_0000_0000_0000_0000_0000_0006);
// G3.
/// Seeded person "Lina" (G3) — linked to bob user.
pub const SEED_PERSON_LINA_ID: Uuid = Uuid::from_u128(0x0000_0003_0000_0000_0000_0000_0000_0007);
/// Seeded person "Max" (G3).
pub const SEED_PERSON_MAX_ID: Uuid = Uuid::from_u128(0x0000_0003_0000_0000_0000_0000_0000_0008);

/// Outcome of a `run_seed` invocation. Both counts always reflect what the
/// seed inserted/updated for this run (not "newly inserted"), since the UPSERTs
/// touch every row unconditionally.
#[derive(Debug)]
pub struct SeedReport {
    /// Number of users upserted (always 3 on a successful run).
    pub users_upserted: usize,
    /// Number of persons upserted (always 8 on a successful run).
    pub persons_upserted: usize,
    /// One `(email, consume_url)` pair per seeded user, in admin/alice/bob order.
    pub magic_links: Vec<(String, String)>,
}

/// Run the deterministic seed against `pool`. Idempotent: a second invocation
/// against the same DB leaves row counts identical.
///
/// Mints one fresh single-use magic-link URL per seeded user (login purpose)
/// using the shared `mint_magic_link_url` helper, so the URLs match the real
/// `/auth/magic-link` flow byte-for-byte. The returned URLs are intended for
/// the seeder's stdout / docker logs — paste into the browser to sign in as
/// admin / alice / bob.
///
/// # Errors
/// Propagates any Postgres error from the upsert statements or magic-link mint.
pub async fn run_seed(pool: &PgPool, cfg: &Config) -> anyhow::Result<SeedReport> {
    seed_users(pool).await.context("seed users")?;
    seed_family(pool).await.context("seed family")?;
    seed_memberships(pool).await.context("seed family_memberships")?;
    seed_persons(pool).await.context("seed persons")?;
    seed_parent_links(pool).await.context("seed parent_links")?;
    seed_partnerships(pool).await.context("seed partnerships")?;

    let magic_links_repo: Arc<dyn MagicLinkRepo> = Arc::new(PgMagicLinkRepo::new(pool.clone()));
    let magic_links = mint_magic_links(&magic_links_repo, cfg).await.context("mint magic links")?;

    Ok(SeedReport { users_upserted: 3, persons_upserted: 8, magic_links })
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

async fn seed_persons(pool: &PgPool) -> anyhow::Result<()> {
    // (id, given, family, linked_user_id_opt).
    let rows: [(Uuid, &str, &str, Option<Uuid>); 8] = [
        (SEED_PERSON_OTTO_ID, "Otto", "Müller", None),
        (SEED_PERSON_HANNELORE_ID, "Hannelore", "Müller", None),
        (SEED_PERSON_WERNER_ID, "Werner", "Schmidt", None),
        (SEED_PERSON_GRETA_ID, "Greta", "Schmidt", None),
        (SEED_PERSON_KLAUS_ID, "Klaus", "Müller", Some(SEED_ADMIN_USER_ID)),
        (SEED_PERSON_ANNA_ID, "Anna", "Müller", Some(SEED_ALICE_USER_ID)),
        (SEED_PERSON_LINA_ID, "Lina", "Müller", Some(SEED_BOB_USER_ID)),
        (SEED_PERSON_MAX_ID, "Max", "Müller", None),
    ];
    for (id, given, family, linked) in rows {
        sqlx::query(
            "INSERT INTO persons (id, family_id, given_name, family_name, linked_user_id) \
             VALUES ($1, $2, $3, $4, $5) \
             ON CONFLICT (id) DO UPDATE SET \
                 family_id = EXCLUDED.family_id, \
                 given_name = EXCLUDED.given_name, \
                 family_name = EXCLUDED.family_name, \
                 linked_user_id = EXCLUDED.linked_user_id",
        )
        .bind(id)
        .bind(SEED_FAMILY_ID)
        .bind(given)
        .bind(family)
        .bind(linked)
        .execute(pool)
        .await?;
    }
    Ok(())
}

async fn seed_parent_links(pool: &PgPool) -> anyhow::Result<()> {
    // (child_id, parent_id). All biological. Tree is acyclic by construction.
    let edges: [(Uuid, Uuid); 8] = [
        // Klaus's parents: Otto + Hannelore.
        (SEED_PERSON_KLAUS_ID, SEED_PERSON_OTTO_ID),
        (SEED_PERSON_KLAUS_ID, SEED_PERSON_HANNELORE_ID),
        // Anna's parents: Werner + Greta.
        (SEED_PERSON_ANNA_ID, SEED_PERSON_WERNER_ID),
        (SEED_PERSON_ANNA_ID, SEED_PERSON_GRETA_ID),
        // Lina's parents: Klaus + Anna.
        (SEED_PERSON_LINA_ID, SEED_PERSON_KLAUS_ID),
        (SEED_PERSON_LINA_ID, SEED_PERSON_ANNA_ID),
        // Max's parents: Klaus + Anna.
        (SEED_PERSON_MAX_ID, SEED_PERSON_KLAUS_ID),
        (SEED_PERSON_MAX_ID, SEED_PERSON_ANNA_ID),
    ];
    for (child, parent) in edges {
        sqlx::query(
            "INSERT INTO parent_links (child_id, parent_id, kind, note) \
             VALUES ($1, $2, 'biological'::parent_link_kind, '') \
             ON CONFLICT (child_id, parent_id) DO UPDATE SET \
                 kind = EXCLUDED.kind, \
                 note = EXCLUDED.note",
        )
        .bind(child)
        .bind(parent)
        .execute(pool)
        .await?;
    }
    Ok(())
}

async fn seed_partnerships(pool: &PgPool) -> anyhow::Result<()> {
    // (partner_a_id, partner_b_id, kind). The CHECK constraint requires a < b.
    // We pre-order each pair so the seed data satisfies it byte-for-byte.
    let (otto, hannelore) = order_pair(SEED_PERSON_OTTO_ID, SEED_PERSON_HANNELORE_ID);
    let (werner, greta) = order_pair(SEED_PERSON_WERNER_ID, SEED_PERSON_GRETA_ID);
    let (klaus, anna) = order_pair(SEED_PERSON_KLAUS_ID, SEED_PERSON_ANNA_ID);
    let rows: [(Uuid, Uuid, &str); 3] =
        [(otto, hannelore, "marriage"), (werner, greta, "marriage"), (klaus, anna, "civil_union")];
    for (a, b, kind) in rows {
        // The `partnerships_unique_open` partial index on (a, b, kind) WHERE
        // ended_on IS NULL is a UNIQUE INDEX, not a table constraint, so the
        // ON CONFLICT target is the column-list form. Postgres matches by the
        // partial index automatically when the inserted row also satisfies the
        // predicate (ended_on IS NULL — we never set it).
        sqlx::query(
            "INSERT INTO partnerships (family_id, partner_a_id, partner_b_id, kind, note) \
             VALUES ($1, $2, $3, ($4::text)::partnership_kind, '') \
             ON CONFLICT (partner_a_id, partner_b_id, kind) \
                 WHERE ended_on IS NULL \
             DO UPDATE SET note = EXCLUDED.note",
        )
        .bind(SEED_FAMILY_ID)
        .bind(a)
        .bind(b)
        .bind(kind)
        .execute(pool)
        .await?;
    }
    Ok(())
}

/// Return `(min, max)` of the two UUIDs so partnership rows satisfy the
/// `partner_a_id < partner_b_id` `CHECK`.
fn order_pair(a: Uuid, b: Uuid) -> (Uuid, Uuid) {
    if a < b { (a, b) } else { (b, a) }
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
            &cfg.web_public_url,
            cfg.magic_link_ttl_seconds,
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
    use std::sync::Arc;
    use std::time::Duration;

    use my_family_persistence::Database;
    use sqlx::Row;
    use testcontainers::ContainerAsync;
    use testcontainers::runners::AsyncRunner;
    use testcontainers_modules::postgres::Postgres;

    use super::*;
    use crate::{AppEnv, LogFormat};

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
            log_format: LogFormat::Pretty,
            rust_log: "info".into(),
            api_host: "0.0.0.0".into(),
            api_port: 8080,
            api_public_url: "http://localhost:8080".into(),
            web_public_url: "http://my-family.docker".into(),
            cors_allowed_origins: "http://localhost:5173".into(),
            api_enable_docs: false,
            api_metrics_bind: "0.0.0.0:9090".into(),
            database_url: url.clone(),
            database_max_connections: 4,
            database_acquire_timeout_seconds: 5,
            database_statement_timeout_ms: 30_000,
            redis_url: "redis://localhost".into(),
            redis_max_connections: 4,
            redis_key_prefix: "t:".into(),
            jwt_private_key: "x".into(),
            jwt_private_key_id: "t".into(),
            jwt_public_keys: "[]".into(),
            jwt_issuer: "iss".into(),
            jwt_audience: "aud".into(),
            jwt_access_ttl_seconds: 900,
            jwt_refresh_ttl_seconds: 86_400,
            jwt_refresh_absolute_ttl_seconds: 604_800,
            cookie_domain: String::new(),
            cookie_secure: false,
            cookie_samesite_access: "Lax".into(),
            cookie_samesite_refresh: "Strict".into(),
            magic_link_ttl_seconds: 900,
            invite_ttl_seconds: 1_209_600,
            magic_link_rate_per_email_per_hour: 10,
            magic_link_rate_per_ip_per_hour: 100,
            email_dsn: "smtp://localhost:1025".into(),
            email_from_name: "t".into(),
            email_from_address: "no-reply@t".into(),
            email_reply_to: None,
            email_timeout_seconds: 10,
        };
        let pool = db.pool().clone();
        Harness { pool, cfg, _pg: pg }
    }

    async fn count(pool: &sqlx::PgPool, table: &str) -> i64 {
        let q = format!("SELECT count(*) FROM {table}");
        let row = sqlx::query(&q).fetch_one(pool).await.expect("count");
        row.get::<i64, _>(0)
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn seed_against_empty_db_inserts_expected_row_counts() {
        let h = start_pg().await;
        let report = run_seed(&h.pool, &h.cfg).await.expect("seed");

        assert_eq!(report.users_upserted, 3);
        assert_eq!(report.persons_upserted, 8);
        assert_eq!(count(&h.pool, "users").await, 3);
        assert_eq!(count(&h.pool, "families").await, 1);
        assert_eq!(count(&h.pool, "family_memberships").await, 3);
        assert_eq!(count(&h.pool, "persons").await, 8);
        assert_eq!(count(&h.pool, "parent_links").await, 8);
        assert_eq!(count(&h.pool, "partnerships").await, 3);

        assert_eq!(report.magic_links.len(), 3);
        for (email, url) in &report.magic_links {
            assert!(!email.is_empty(), "magic-link email must be non-empty");
            assert!(url.contains("/auth/consume?token="), "url must point at consume: {url}");
            assert!(url.starts_with("http://my-family.docker"), "url uses web_public_url: {url}");
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn seed_is_idempotent_on_row_counts() {
        let h = start_pg().await;
        let _ = run_seed(&h.pool, &h.cfg).await.expect("first seed");
        let r2 = run_seed(&h.pool, &h.cfg).await.expect("second seed");

        // Row counts unchanged after a second invocation.
        assert_eq!(count(&h.pool, "users").await, 3);
        assert_eq!(count(&h.pool, "families").await, 1);
        assert_eq!(count(&h.pool, "family_memberships").await, 3);
        assert_eq!(count(&h.pool, "persons").await, 8);
        assert_eq!(count(&h.pool, "parent_links").await, 8);
        assert_eq!(count(&h.pool, "partnerships").await, 3);

        // The second invocation still mints fresh magic links (one per user).
        // Magic-link tokens are append-only — count rises across calls.
        assert_eq!(r2.magic_links.len(), 3);
        assert_eq!(count(&h.pool, "magic_link_tokens").await, 6);
    }

    #[test]
    fn order_pair_returns_min_max() {
        let small = Uuid::from_u128(1);
        let big = Uuid::from_u128(2);
        assert_eq!(order_pair(small, big), (small, big));
        assert_eq!(order_pair(big, small), (small, big));
    }

    // Silence unused-import on Arc when the harness compiles without it.
    #[allow(dead_code)]
    fn _arc_keep_alive(_: Arc<()>) {}
}
