//! `parent_links` and `partnerships` for the dev seed.
//!
//! Both tables are upserted via `ON CONFLICT (…) DO UPDATE` so a re-seed
//! is idempotent. Partnership rows are keyed by hardcoded `id` (rather
//! than the partial-unique `(a, b, kind) WHERE ended_on IS NULL` index)
//! so closed/widowed rows can be re-seeded without inserting duplicates.

mod parent_links;
mod partnerships;

use sqlx::PgPool;
use time::Date;
use uuid::Uuid;

use crate::ids::{
    SEED_FAMILY_ID, SEED_PARTNERSHIP_FLK_EDGAR_GISELA_ID, SEED_PARTNERSHIP_FRIEDRICH_LOTTE_ID,
    SEED_PARTNERSHIP_HEINZ_URSULA_ID, SEED_PARTNERSHIP_K_HUBERT_SARA_ID,
    SEED_PARTNERSHIP_K_REINHARDT_HELGA_ID, SEED_PARTNERSHIP_K_TIM_MIA_ID,
    SEED_PARTNERSHIP_KLAUS_ANNA_ID, SEED_PARTNERSHIP_KLAUS_BRIGITTE_ID,
    SEED_PARTNERSHIP_KLAUS_KARIN_ID, SEED_PARTNERSHIP_KLAUS_YUKI_ID,
    SEED_PARTNERSHIP_LARS_METTE_ID, SEED_PARTNERSHIP_OTTO_HANNELORE_ID,
    SEED_PARTNERSHIP_SABINE_JULIA_ID, SEED_PARTNERSHIP_STB_CARLA_TOBIAS_ID,
    SEED_PARTNERSHIP_STB_FELIX_BEATE_ID, SEED_PARTNERSHIP_STB_HARTMUT_MARGARETE_ID,
    SEED_PARTNERSHIP_SVEN_MAREN_ID, SEED_PARTNERSHIP_WERNER_GRETA_ID,
};

/// One `(child, parent)` row plus the relationship kind.
///
/// `biological` covers the canonical case. `step` / `adoptive` /
/// `legal` / `social` are for the edge-case fixtures (Felix's
/// stepmother, Lena's adoptive parents, etc.).
pub(crate) struct ParentLinkSeed {
    pub(crate) child: Uuid,
    pub(crate) parent: Uuid,
    pub(crate) kind: &'static str,
}

/// Upsert one `parent_link` row. Shared inserter so the anonymized
/// prod-mirror families ([`crate::vellmar`], [`crate::lang`]) supply
/// only their own data. `parent_links` has no `family_id` column — it's
/// scoped implicitly through the `persons` it references.
///
/// # Errors
/// Propagates any Postgres error from the `INSERT … ON CONFLICT`.
pub(crate) async fn upsert_parent_link(pool: &PgPool, r: &ParentLinkSeed) -> anyhow::Result<()> {
    sqlx::query(
        "INSERT INTO parent_links (child_id, parent_id, kind, note) \
         VALUES ($1, $2, ($3::text)::parent_link_kind, '') \
         ON CONFLICT (child_id, parent_id) DO UPDATE SET \
             kind = EXCLUDED.kind, \
             note = EXCLUDED.note",
    )
    .bind(r.child)
    .bind(r.parent)
    .bind(r.kind)
    .execute(pool)
    .await?;
    Ok(())
}

/// Upsert every seeded `parent_link` row.
///
/// # Errors
/// Propagates any Postgres error from the `INSERT … ON CONFLICT … DO
/// UPDATE` statements.
pub async fn seed_parent_links(pool: &PgPool) -> anyhow::Result<()> {
    for r in &parent_links::PARENT_LINKS {
        upsert_parent_link(pool, r).await?;
    }
    Ok(())
}

/// One partnership row. `started_on` + `ended_on` + `end_reason` cover
/// the open / divorced / widowed cases the FE relations panel renders.
pub(crate) struct PartnershipSeed {
    pub(crate) id: Uuid,
    pub(crate) partner_a: Uuid,
    pub(crate) partner_b: Uuid,
    pub(crate) kind: &'static str,
    pub(crate) started_on: Option<Date>,
    pub(crate) ended_on: Option<Date>,
    pub(crate) end_reason: Option<&'static str>,
    pub(crate) note: &'static str,
}

#[allow(
    clippy::panic,
    reason = "const-fn date constructor: arguments are static literals validated at build time"
)]
const fn ymd(y: i32, m: u8, d: u8) -> Date {
    match my_fam_tree_domain::dates::date_from_ymd(y, m, d) {
        Some(date) => date,
        None => panic!("static seed date must be valid"),
    }
}

/// Upsert every seeded partnership row.
///
/// # Errors
/// Propagates any Postgres error from the `INSERT … ON CONFLICT (id) DO
/// UPDATE` statements.
pub async fn seed_partnerships(pool: &PgPool) -> anyhow::Result<()> {
    // Wipe any partnerships in the seeded family that ARE NOT one of the
    // hardcoded seed rows. Reason: the historical seed inserted Otto +
    // Hannelore etc. with `gen_random_uuid()` ids and relied on a partial
    // unique index for upsert. Switching to hardcoded `id` upserts means
    // those legacy random-id rows would now conflict on the partial index
    // `partnerships_unique_open (a, b, kind) WHERE ended_on IS NULL`. The
    // DELETE also nukes any user-added partnerships on a `seeder` re-run
    // — that's deliberate, the seeder is a reset.
    let seed_ids: [Uuid; 18] = [
        SEED_PARTNERSHIP_OTTO_HANNELORE_ID,
        SEED_PARTNERSHIP_WERNER_GRETA_ID,
        SEED_PARTNERSHIP_KLAUS_ANNA_ID,
        SEED_PARTNERSHIP_FRIEDRICH_LOTTE_ID,
        SEED_PARTNERSHIP_KLAUS_BRIGITTE_ID,
        SEED_PARTNERSHIP_SABINE_JULIA_ID,
        SEED_PARTNERSHIP_KLAUS_KARIN_ID,
        SEED_PARTNERSHIP_KLAUS_YUKI_ID,
        SEED_PARTNERSHIP_SVEN_MAREN_ID,
        SEED_PARTNERSHIP_HEINZ_URSULA_ID,
        SEED_PARTNERSHIP_LARS_METTE_ID,
        SEED_PARTNERSHIP_K_HUBERT_SARA_ID,
        SEED_PARTNERSHIP_K_REINHARDT_HELGA_ID,
        SEED_PARTNERSHIP_K_TIM_MIA_ID,
        SEED_PARTNERSHIP_STB_HARTMUT_MARGARETE_ID,
        SEED_PARTNERSHIP_STB_CARLA_TOBIAS_ID,
        SEED_PARTNERSHIP_STB_FELIX_BEATE_ID,
        SEED_PARTNERSHIP_FLK_EDGAR_GISELA_ID,
    ];
    sqlx::query("DELETE FROM partnerships WHERE family_id = $1 AND id <> ALL($2)")
        .bind(SEED_FAMILY_ID)
        .bind(&seed_ids[..])
        .execute(pool)
        .await?;

    for p in &partnerships::PARTNERSHIPS {
        upsert_partnership(pool, SEED_FAMILY_ID, p).await?;
    }
    Ok(())
}

/// Delete every partnership in `family_id` whose id is not in `keep`,
/// then return — the inverse-cleanup half of a family's partnership
/// reset. Shared so each seeded family resets only its own rows.
///
/// # Errors
/// Propagates any Postgres error from the `DELETE`.
pub(crate) async fn delete_non_seed_partnerships(
    pool: &PgPool,
    family_id: Uuid,
    keep: &[Uuid],
) -> anyhow::Result<()> {
    sqlx::query("DELETE FROM partnerships WHERE family_id = $1 AND id <> ALL($2)")
        .bind(family_id)
        .bind(keep)
        .execute(pool)
        .await?;
    Ok(())
}

/// Upsert one partnership row into `family_id`. Shared inserter so the
/// anonymized prod-mirror families supply only their own data.
///
/// # Errors
/// Propagates any Postgres error from the `INSERT … ON CONFLICT`.
pub(crate) async fn upsert_partnership(
    pool: &PgPool,
    family_id: Uuid,
    p: &PartnershipSeed,
) -> anyhow::Result<()> {
    sqlx::query(
        "INSERT INTO partnerships \
             (id, family_id, partner_a_id, partner_b_id, kind, started_on, ended_on, \
              end_reason, note) \
         VALUES ($1, $2, $3, $4, ($5::text)::partnership_kind, $6, $7, \
                 $8::text::partnership_end_reason, $9) \
         ON CONFLICT (id) DO UPDATE SET \
             family_id = EXCLUDED.family_id, \
             partner_a_id = EXCLUDED.partner_a_id, \
             partner_b_id = EXCLUDED.partner_b_id, \
             kind = EXCLUDED.kind, \
             started_on = EXCLUDED.started_on, \
             ended_on = EXCLUDED.ended_on, \
             end_reason = EXCLUDED.end_reason, \
             note = EXCLUDED.note",
    )
    .bind(p.id)
    .bind(family_id)
    .bind(p.partner_a)
    .bind(p.partner_b)
    .bind(p.kind)
    .bind(p.started_on)
    .bind(p.ended_on)
    .bind(p.end_reason)
    .bind(p.note)
    .execute(pool)
    .await?;
    Ok(())
}

/// Return `(min, max)` of the two UUIDs so partnership rows satisfy the
/// `partner_a_id < partner_b_id` CHECK constraint.
pub(crate) const fn order_pair(a: Uuid, b: Uuid) -> (Uuid, Uuid) {
    if a.as_u128() < b.as_u128() { (a, b) } else { (b, a) }
}

#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use super::order_pair;

    #[test]
    fn order_pair_returns_min_max() {
        let a = Uuid::from_u128(1);
        let b = Uuid::from_u128(2);
        assert_eq!(order_pair(a, b), (a, b));
        assert_eq!(order_pair(b, a), (a, b));
    }
}
