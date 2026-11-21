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
use my_fam_tree_persistence::seed::SeedDb;
use uuid::Uuid;

pub mod contacts;
pub mod ids;
pub mod lang;
pub mod persons;
pub mod relationships;
pub mod vellmar;

// Re-export the public UUIDs so call sites (and tests) keep the old import
// path `my_fam_tree_seeder::SEED_…_ID` working.
pub use ids::{
    SEED_ADMIN_USER_ID, SEED_ALICE_USER_ID, SEED_BOB_USER_ID, SEED_FAMILY_ID, SEED_FAMILY_LANG_ID,
    SEED_FAMILY_VELLMAR_ID, SEED_PARTNERSHIP_FRIEDRICH_LOTTE_ID, SEED_PARTNERSHIP_KLAUS_ANNA_ID,
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

/// Run the deterministic seed through `db` (the configured pool). Idempotent: a second
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
pub async fn run_seed(db: &SeedDb<'_>, cfg: &Config) -> anyhow::Result<SeedReport> {
    seed_users(db).await.context("seed users")?;
    seed_families(db).await.context("seed families")?;
    seed_memberships(db).await.context("seed family_memberships")?;
    persons::seed_persons(db).await.context("seed persons")?;
    relationships::seed_parent_links(db).await.context("seed parent_links")?;
    relationships::seed_partnerships(db).await.context("seed partnerships")?;
    contacts::seed_contacts(db).await.context("seed person_contacts")?;
    // Anonymized mirrors of two real prod trees — own persons +
    // relationships, each self-contained and family-scoped.
    vellmar::seed(db).await.context("seed Vellmar family")?;
    lang::seed(db).await.context("seed Lang family")?;

    let magic_links_repo: Arc<dyn MagicLinkRepo> = Arc::new(db.magic_links());
    let magic_links = mint_magic_links(&magic_links_repo, cfg).await.context("mint magic links")?;

    let persons_upserted = SEED_PERSON_COUNT + vellmar::PERSON_COUNT + lang::PERSON_COUNT;
    Ok(SeedReport { users_upserted: 3, persons_upserted, magic_links })
}

async fn seed_users(db: &SeedDb<'_>) -> anyhow::Result<()> {
    // (id, email, display_name) tuples. CITEXT email column dedupes on lower-case.
    let rows: [(Uuid, &str, &str); 3] = [
        (SEED_ADMIN_USER_ID, "admin@example.com", "Admin"),
        (SEED_ALICE_USER_ID, "alice@example.com", "Alice"),
        (SEED_BOB_USER_ID, "bob@example.com", "Bob"),
    ];
    for (id, email, display_name) in rows {
        // Reconcile a "squatter": a row holding this seed email under a
        // DIFFERENT id (e.g. the user signed up / was invited with the
        // same address through the app, which mints a random uuid). The
        // upsert below conflicts on `id`, so it can't see that collision
        // and would trip the `users_email_key` unique constraint. Delete
        // the squatter first (its dependent rows cascade — the seeder is
        // an explicit dev/test reset). The seed-id row, if present, is
        // left for the upsert to refresh in place.
        db.delete_email_squatter(email, id).await?;
        db.upsert_user(id, email, display_name).await?;
    }
    Ok(())
}

async fn seed_families(db: &SeedDb<'_>) -> anyhow::Result<()> {
    // (id, name). All three created by the admin user.
    let rows: [(Uuid, &str); 3] = [
        (SEED_FAMILY_ID, "Müller"),
        (ids::SEED_FAMILY_VELLMAR_ID, "Vellmar"),
        (ids::SEED_FAMILY_LANG_ID, "Lang"),
    ];
    for (id, name) in rows {
        db.upsert_family(id, name, SEED_ADMIN_USER_ID).await?;
    }
    Ok(())
}

async fn seed_memberships(db: &SeedDb<'_>) -> anyhow::Result<()> {
    // Every seeded user is a member of every seeded family so all three
    // trees are switchable in the app. (user_id, role) per family.
    let members: [(Uuid, &str); 3] =
        [(SEED_ADMIN_USER_ID, "owner"), (SEED_ALICE_USER_ID, "admin"), (SEED_BOB_USER_ID, "user")];
    let families = [SEED_FAMILY_ID, ids::SEED_FAMILY_VELLMAR_ID, ids::SEED_FAMILY_LANG_ID];
    for family_id in families {
        for (user_id, role) in members {
            db.upsert_membership(family_id, user_id, role).await?;
        }
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
            my_fam_tree_email::Locale::En,
            cfg.magic_link.ttl_seconds,
        )
        .await?;
        out.push((email.to_string(), url));
    }
    Ok(out)
}
