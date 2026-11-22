//! Third seeded family — "Lang".
//!
//! An ANONYMIZED mirror of a real production tree that has NO partnership
//! rows: co-parents are linked ONLY through their shared children. This
//! reproduces the layout case where two people who clearly parented a
//! child together are NOT joined by a partnership edge, so the engine
//! sees them as independent singletons. Names + places changed; the
//! structure and birth dates are preserved.
//!
//! Shape:
//!   Egon + Kerstin → Reto                 (Reto's parents)
//!   Sigrun + Reto  → Alma Diana           (no partnership edge)
//!   Sigrun + Detlef → Svea                (no partnership edge)
//!   Svea → Lina Jo                        (single parent)
//!
//! Sigrun co-parents with two different partners (Reto, Detlef) without
//! any partnership row — the missing-edge layout repro.

use my_fam_tree_persistence::seed::SeedDb;
use uuid::Uuid;

use crate::ids::SEED_FAMILY_LANG_ID;
use crate::persons::{PersonSeed, delete_non_seed_persons, upsert_person, ymd};
use crate::relationships::{ParentLinkSeed, upsert_parent_link};

const P_EGON: Uuid = Uuid::from_u128(0x0000_0003_0000_0000_0000_0000_0000_0601); // Reto's father
const P_KERSTIN: Uuid = Uuid::from_u128(0x0000_0003_0000_0000_0000_0000_0000_0602); // Reto's mother
const P_RETO: Uuid = Uuid::from_u128(0x0000_0003_0000_0000_0000_0000_0000_0603);
const P_SIGRUN: Uuid = Uuid::from_u128(0x0000_0003_0000_0000_0000_0000_0000_0604);
const P_DETLEF: Uuid = Uuid::from_u128(0x0000_0003_0000_0000_0000_0000_0000_0605);
const P_ALMA_DIANA: Uuid = Uuid::from_u128(0x0000_0003_0000_0000_0000_0000_0000_0606);
const P_SVEA: Uuid = Uuid::from_u128(0x0000_0003_0000_0000_0000_0000_0000_0607);
const P_LINA_JO: Uuid = Uuid::from_u128(0x0000_0003_0000_0000_0000_0000_0000_0608);

/// Number of persons this family seeds (summed into the seed report).
pub const PERSON_COUNT: usize = 8;
/// Number of parent-link rows this family seeds (row-count assert only).
pub const PARENT_LINK_COUNT: i64 = 7;

#[expect(clippy::too_many_lines, reason = "static fixture table; splitting hurts readability")]
const fn persons() -> [PersonSeed; PERSON_COUNT] {
    [
        PersonSeed {
            id: P_EGON,
            given: "Egon",
            family: "Lang",
            name_at_birth: "",
            nickname: "",
            gender: "Male",
            // No birth date in prod; pick one older than Reto (1964).
            birth_date: ymd(1938, 1, 1),
            birth_place: "Kiel",
            death_date: Some(ymd(2011, 5, 31)),
            notes: "Reto's father.",
            linked_user_id: None,
        },
        PersonSeed {
            id: P_KERSTIN,
            given: "Kerstin",
            family: "Lang",
            name_at_birth: "Ohlsen",
            nickname: "",
            gender: "Female",
            birth_date: ymd(1940, 1, 1),
            birth_place: "Kiel",
            death_date: Some(ymd(2022, 5, 31)),
            notes: "Reto's mother.",
            linked_user_id: None,
        },
        PersonSeed {
            id: P_RETO,
            given: "Reto",
            family: "Lang",
            name_at_birth: "",
            nickname: "",
            gender: "Male",
            birth_date: ymd(1964, 11, 14),
            birth_place: "Kiel",
            death_date: None,
            notes: "Egon + Kerstin son; co-parent of Alma Diana (no partnership edge).",
            linked_user_id: None,
        },
        PersonSeed {
            id: P_SIGRUN,
            given: "Sigrun",
            family: "Schober",
            name_at_birth: "",
            nickname: "",
            gender: "Female",
            birth_date: ymd(1965, 5, 31),
            birth_place: "",
            death_date: None,
            notes: "Co-parents Alma Diana with Reto + Svea with Detlef; no partnership rows.",
            linked_user_id: None,
        },
        PersonSeed {
            id: P_DETLEF,
            given: "Detlef",
            family: "Bracker",
            name_at_birth: "",
            nickname: "",
            gender: "Male",
            birth_date: ymd(1968, 1, 1),
            birth_place: "",
            death_date: None,
            notes: "Co-parent of Svea with Sigrun (no partnership edge).",
            linked_user_id: None,
        },
        PersonSeed {
            id: P_ALMA_DIANA,
            given: "Alma Diana",
            family: "Schober",
            name_at_birth: "",
            nickname: "",
            gender: "Female",
            birth_date: ymd(1989, 4, 6),
            birth_place: "Flensburg",
            death_date: None,
            notes: "Sigrun + Reto daughter; same individual as the Vellmar family's Alma.",
            linked_user_id: None,
        },
        PersonSeed {
            id: P_SVEA,
            given: "Svea",
            family: "Schober",
            name_at_birth: "",
            nickname: "",
            gender: "Female",
            birth_date: ymd(1994, 8, 3),
            birth_place: "Flensburg",
            death_date: None,
            notes: "Sigrun + Detlef daughter; mother of Lina Jo.",
            linked_user_id: None,
        },
        PersonSeed {
            id: P_LINA_JO,
            given: "Lina Jo",
            family: "Schober",
            name_at_birth: "",
            nickname: "",
            gender: "Female",
            birth_date: ymd(2017, 1, 23),
            birth_place: "Flensburg",
            death_date: None,
            notes: "Svea's daughter (single parent in the seed).",
            linked_user_id: None,
        },
    ]
}

fn parent_links() -> Vec<ParentLinkSeed> {
    let bio = |child: Uuid, parent: Uuid| ParentLinkSeed { child, parent, kind: "biological" };
    vec![
        bio(P_RETO, P_EGON),
        bio(P_RETO, P_KERSTIN),
        bio(P_ALMA_DIANA, P_SIGRUN),
        bio(P_ALMA_DIANA, P_RETO),
        bio(P_SVEA, P_SIGRUN),
        bio(P_SVEA, P_DETLEF),
        bio(P_LINA_JO, P_SVEA),
    ]
}

/// Seed the Lang family: persons + parent links only (no partnerships,
/// matching the prod tree this mirrors). Resets its own family-scoped
/// person rows first.
///
/// # Errors
/// Propagates any Postgres error from the upserts.
pub async fn seed(db: &SeedDb<'_>) -> anyhow::Result<()> {
    let rows = persons();
    let keep: Vec<Uuid> = rows.iter().map(|p| p.id).collect();
    delete_non_seed_persons(db, SEED_FAMILY_LANG_ID, &keep).await?;
    for p in &rows {
        upsert_person(db, SEED_FAMILY_LANG_ID, p).await?;
    }
    for r in &parent_links() {
        upsert_parent_link(db, r).await?;
    }
    Ok(())
}
