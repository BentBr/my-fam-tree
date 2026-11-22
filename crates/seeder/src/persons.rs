//! Person table for the dev seed.
//!
//! 22 rows across 4 generations covering the relationship edge cases we
//! want exercised in dev / e2e:
//!
//! - Widowed: Hannelore (Otto died 2010) and Friedrich (Lotte died 2018).
//! - Divorced + remarried: Klaus had Brigitte before Anna.
//! - Half-siblings: Felix (Klaus + Brigitte) vs. Lina/Max/Mia (Klaus + Anna).
//! - Step-parent: Anna as `step` parent of Felix.
//! - Same-sex partnership: Sabine + Julia (`civil_union`).
//! - Adoption: Sabine + Julia adopted Lena (both `adoptive`).
//! - Single parent: Markus raised Tom on his own (no partnership row).
//! - Multi-sibling clusters: Sabine is Anna's bio sister; Klaus + Anna
//!   have three biological children together (Lina, Max, Mia).
//! - 4-generation depth: Lina's children (Emma, Noah) are G4.
//!
//! Phase 3 dropped the flat contact columns from `persons` — contact
//! data now lives in the dedicated `person_contacts` table, seeded in
//! the [`crate::contacts`] module.

mod descendants;
mod elders;
mod falke_krause;
mod steinbach;

use my_fam_tree_persistence::seed::{PersonRow, SeedDb};
use time::Date;
use uuid::Uuid;

use crate::ids::SEED_FAMILY_ID;

/// Static seed of every person field.
///
/// Dates satisfy the `validation::relationships` hard rules (parent older
/// than child; biological parents alive at conception; partnership starts
/// after both partners' birthdays) so future flows that re-run validation
/// against the seeded graph never fail.
pub(crate) struct PersonSeed {
    pub(crate) id: Uuid,
    pub(crate) given: &'static str,
    pub(crate) family: &'static str,
    pub(crate) name_at_birth: &'static str,
    pub(crate) nickname: &'static str,
    pub(crate) gender: &'static str,
    pub(crate) birth_date: Date,
    pub(crate) birth_place: &'static str,
    pub(crate) death_date: Option<Date>,
    pub(crate) notes: &'static str,
    pub(crate) linked_user_id: Option<Uuid>,
}

#[expect(
    clippy::panic,
    reason = "const-fn date constructor: arguments are static literals validated at build time"
)]
pub(crate) const fn ymd(y: i32, m: u8, d: u8) -> Date {
    match my_fam_tree_domain::dates::date_from_ymd(y, m, d) {
        Some(date) => date,
        None => panic!("static seed date must be valid"),
    }
}

/// Upsert every seeded person row.
///
/// # Errors
/// Propagates any Postgres error from the `INSERT … ON CONFLICT … DO
/// UPDATE` statements.
pub async fn seed_persons(db: &SeedDb<'_>) -> anyhow::Result<()> {
    let rows: Vec<&PersonSeed> = [
        elders::ROWS.as_slice(),
        descendants::ROWS.as_slice(),
        steinbach::ROWS.as_slice(),
        falke_krause::ROWS.as_slice(),
    ]
    .into_iter()
    .flatten()
    .collect();

    // Wipe any person in the seeded family that isn't one of the canonical
    // seed rows. Cleans up "relict" rows left behind when a previous seed
    // revision inserted persons that have since been removed (e.g. an
    // earlier Wagner/Falke side-branch repro). The same `ON DELETE
    // CASCADE` rules that drop a person's parent_links / partnerships /
    // person_contacts / person_favourites when the row goes away apply
    // here, so a single statement is enough to clean the dependent graph.
    //
    // The seeder is explicitly a RESET for the seeded family — this also
    // nukes any user-added person rows on a re-seed, matching the
    // partnership-reset behaviour already in `seed_partnerships`.
    let seed_ids: Vec<Uuid> = rows.iter().map(|p| p.id).collect();
    delete_non_seed_persons(db, SEED_FAMILY_ID, &seed_ids).await?;

    for p in &rows {
        upsert_person(db, SEED_FAMILY_ID, p).await?;
    }
    Ok(())
}

/// Wipe every person in `family_id` whose id is not in `keep`. The
/// `ON DELETE CASCADE` rules on `parent_links` / `partnerships` /
/// `person_contacts` / `person_favourites` drop the dependent graph in
/// the same statement. Shared by all seeded families (Müller + the
/// anonymized prod mirrors in [`crate::vellmar`] / [`crate::lang`]).
///
/// # Errors
/// Propagates any Postgres error from the `DELETE`.
pub(crate) async fn delete_non_seed_persons(
    db: &SeedDb<'_>,
    family_id: Uuid,
    keep: &[Uuid],
) -> anyhow::Result<()> {
    db.delete_persons_except(family_id, keep).await?;
    Ok(())
}

/// Upsert one person row into `family_id`. Shared inserter so each
/// seeded family supplies only its own [`PersonSeed`] data.
///
/// # Errors
/// Propagates any Postgres error from the `INSERT … ON CONFLICT`.
pub(crate) async fn upsert_person(
    db: &SeedDb<'_>,
    family_id: Uuid,
    p: &PersonSeed,
) -> anyhow::Result<()> {
    let row = PersonRow {
        id: p.id,
        given_name: p.given,
        family_name: p.family,
        name_at_birth: p.name_at_birth,
        nickname: p.nickname,
        gender: p.gender,
        birth_date: p.birth_date,
        birth_place: p.birth_place,
        death_date: p.death_date,
        notes: p.notes,
        linked_user_id: p.linked_user_id,
    };
    db.upsert_person(family_id, &row).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::{descendants, elders, falke_krause, steinbach};
    use crate::ids::SEED_PERSON_COUNT;

    #[test]
    fn seed_table_has_the_expected_number_of_unique_persons() {
        let ids: Vec<_> = [
            elders::ROWS.as_slice(),
            descendants::ROWS.as_slice(),
            steinbach::ROWS.as_slice(),
            falke_krause::ROWS.as_slice(),
        ]
        .into_iter()
        .flatten()
        .map(|p| p.id)
        .collect();
        assert_eq!(ids.len(), SEED_PERSON_COUNT);
        assert_eq!(ids.iter().collect::<HashSet<_>>().len(), SEED_PERSON_COUNT);
    }
}
