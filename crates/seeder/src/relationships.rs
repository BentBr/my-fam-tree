//! `parent_links` and `partnerships` for the dev seed.
//!
//! Both tables are upserted via `ON CONFLICT (…) DO UPDATE` so a re-seed
//! is idempotent. Partnership rows are keyed by hardcoded `id` (rather
//! than the partial-unique `(a, b, kind) WHERE ended_on IS NULL` index)
//! so closed/widowed rows can be re-seeded without inserting duplicates.

mod parent_links;
mod partnerships;

use my_fam_tree_persistence::seed::{ParentLinkRow, PartnershipRow, SeedDb};
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
pub(crate) async fn upsert_parent_link(db: &SeedDb<'_>, r: &ParentLinkSeed) -> anyhow::Result<()> {
    db.upsert_parent_link(&ParentLinkRow { child: r.child, parent: r.parent, kind: r.kind })
        .await?;
    Ok(())
}

/// The Müller family's parent-link rows, for tests that reason about the
/// seeded graph without a database.
#[cfg(test)]
pub(crate) const fn parent_link_rows() -> &'static [ParentLinkSeed] {
    &parent_links::PARENT_LINKS
}

/// The Müller family's partnership rows, for tests that reason about the
/// seeded graph without a database.
#[cfg(test)]
pub(crate) const fn partnership_rows() -> &'static [PartnershipSeed] {
    &partnerships::PARTNERSHIPS
}

/// Upsert every seeded `parent_link` row.
///
/// # Errors
/// Propagates any Postgres error from the `INSERT … ON CONFLICT … DO
/// UPDATE` statements.
pub async fn seed_parent_links(db: &SeedDb<'_>) -> anyhow::Result<()> {
    for r in &parent_links::PARENT_LINKS {
        upsert_parent_link(db, r).await?;
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

#[expect(
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
pub async fn seed_partnerships(db: &SeedDb<'_>) -> anyhow::Result<()> {
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
    db.delete_partnerships_except(SEED_FAMILY_ID, &seed_ids).await?;

    for p in &partnerships::PARTNERSHIPS {
        upsert_partnership(db, SEED_FAMILY_ID, p).await?;
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
    db: &SeedDb<'_>,
    family_id: Uuid,
    keep: &[Uuid],
) -> anyhow::Result<()> {
    db.delete_partnerships_except(family_id, keep).await?;
    Ok(())
}

/// Upsert one partnership row into `family_id`. Shared inserter so the
/// anonymized prod-mirror families supply only their own data.
///
/// # Errors
/// Propagates any Postgres error from the `INSERT … ON CONFLICT`.
pub(crate) async fn upsert_partnership(
    db: &SeedDb<'_>,
    family_id: Uuid,
    p: &PartnershipSeed,
) -> anyhow::Result<()> {
    let row = PartnershipRow {
        id: p.id,
        partner_a: p.partner_a,
        partner_b: p.partner_b,
        kind: p.kind,
        started_on: p.started_on,
        ended_on: p.ended_on,
        end_reason: p.end_reason,
        note: p.note,
    };
    db.upsert_partnership(family_id, &row).await?;
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
