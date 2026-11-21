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

use sqlx::PgPool;
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

#[allow(
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
pub async fn seed_persons(pool: &PgPool) -> anyhow::Result<()> {
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
    delete_non_seed_persons(pool, SEED_FAMILY_ID, &seed_ids).await?;

    for p in &rows {
        upsert_person(pool, SEED_FAMILY_ID, p).await?;
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
    pool: &PgPool,
    family_id: Uuid,
    keep: &[Uuid],
) -> anyhow::Result<()> {
    sqlx::query("DELETE FROM persons WHERE family_id = $1 AND id <> ALL($2)")
        .bind(family_id)
        .bind(keep)
        .execute(pool)
        .await?;
    Ok(())
}

/// Upsert one person row into `family_id`. Shared inserter so each
/// seeded family supplies only its own [`PersonSeed`] data.
///
/// # Errors
/// Propagates any Postgres error from the `INSERT … ON CONFLICT`.
pub(crate) async fn upsert_person(
    pool: &PgPool,
    family_id: Uuid,
    p: &PersonSeed,
) -> anyhow::Result<()> {
    sqlx::query(
        "INSERT INTO persons \
             (id, family_id, given_name, family_name, name_at_birth, nickname, gender, \
              birth_date, birth_place, death_date, notes, \
              linked_user_id) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12) \
         ON CONFLICT (id) DO UPDATE SET \
             family_id = EXCLUDED.family_id, \
             given_name = EXCLUDED.given_name, \
             family_name = EXCLUDED.family_name, \
             name_at_birth = EXCLUDED.name_at_birth, \
             nickname = EXCLUDED.nickname, \
             gender = EXCLUDED.gender, \
             birth_date = EXCLUDED.birth_date, \
             birth_place = EXCLUDED.birth_place, \
             death_date = EXCLUDED.death_date, \
             notes = EXCLUDED.notes, \
             linked_user_id = EXCLUDED.linked_user_id",
    )
    .bind(p.id)
    .bind(family_id)
    .bind(p.given)
    .bind(p.family)
    .bind(p.name_at_birth)
    .bind(p.nickname)
    .bind(p.gender)
    .bind(p.birth_date)
    .bind(p.birth_place)
    .bind(p.death_date)
    .bind(p.notes)
    .bind(p.linked_user_id)
    .execute(pool)
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::{descendants, elders, falke_krause, steinbach};

    #[test]
    fn seed_table_has_fifty_unique_persons() {
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
        assert_eq!(ids.len(), 50);
        assert_eq!(ids.iter().collect::<HashSet<_>>().len(), 50);
    }
}
