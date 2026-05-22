//! Person table for the dev seed.
//!
//! 20 rows across 4 generations covering the relationship edge cases we
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

use chrono::NaiveDate;
use sqlx::PgPool;
use uuid::Uuid;

use crate::ids::{
    SEED_ADMIN_USER_ID, SEED_ALICE_USER_ID, SEED_BOB_USER_ID, SEED_FAMILY_ID, SEED_PERSON_ANNA_ID,
    SEED_PERSON_BRIGITTE_ID, SEED_PERSON_EMMA_ID, SEED_PERSON_FELIX_ID, SEED_PERSON_FRIEDRICH_ID,
    SEED_PERSON_GRETA_ID, SEED_PERSON_HANNELORE_ID, SEED_PERSON_JULIA_ID, SEED_PERSON_KLAUS_ID,
    SEED_PERSON_LENA_ID, SEED_PERSON_LINA_ID, SEED_PERSON_LOTTE_ID, SEED_PERSON_MARKUS_ID,
    SEED_PERSON_MAX_ID, SEED_PERSON_MIA_ID, SEED_PERSON_NOAH_ID, SEED_PERSON_OTTO_ID,
    SEED_PERSON_SABINE_ID, SEED_PERSON_TOM_ID, SEED_PERSON_WERNER_ID,
};

/// Static seed of every person field.
///
/// Dates satisfy the `validation::relationships` hard rules (parent older
/// than child; biological parents alive at conception; partnership starts
/// after both partners' birthdays) so future flows that re-run validation
/// against the seeded graph never fail.
struct PersonSeed {
    id: Uuid,
    given: &'static str,
    family: &'static str,
    name_at_birth: &'static str,
    nickname: &'static str,
    gender: &'static str,
    birth_date: NaiveDate,
    birth_place: &'static str,
    death_date: Option<NaiveDate>,
    notes: &'static str,
    linked_user_id: Option<Uuid>,
}

#[allow(
    clippy::panic,
    reason = "const-fn date constructor: arguments are static literals validated at build time"
)]
const fn ymd(y: i32, m: u32, d: u32) -> NaiveDate {
    match NaiveDate::from_ymd_opt(y, m, d) {
        Some(date) => date,
        None => panic!("static seed date must be valid"),
    }
}

/// Upsert every seeded person row.
///
/// # Errors
/// Propagates any Postgres error from the `INSERT … ON CONFLICT … DO
/// UPDATE` statements.
#[allow(clippy::too_many_lines, reason = "static table of 20 persons; splitting hurts readability")]
pub async fn seed_persons(pool: &PgPool) -> anyhow::Result<()> {
    let rows: [PersonSeed; 20] = [
        // -------------------------------------------------------------
        // G1 — Müller line.
        // -------------------------------------------------------------
        PersonSeed {
            id: SEED_PERSON_OTTO_ID,
            given: "Otto",
            family: "Müller",
            name_at_birth: "",
            nickname: "",
            gender: "male",
            birth_date: ymd(1935, 3, 12),
            birth_place: "Hamburg",
            death_date: Some(ymd(2010, 11, 4)),
            notes: "G1 patriarch; worked at the shipyard until retirement.",
            linked_user_id: None,
        },
        PersonSeed {
            id: SEED_PERSON_HANNELORE_ID,
            given: "Hannelore",
            family: "Müller",
            name_at_birth: "Becker",
            nickname: "Hanni",
            gender: "female",
            birth_date: ymd(1938, 7, 23),
            birth_place: "Lübeck",
            death_date: None,
            notes: "Schoolteacher; widowed since 2010, still tends the garden in Hamburg.",
            linked_user_id: None,
        },
        // -------------------------------------------------------------
        // G1 — Schmidt line.
        // -------------------------------------------------------------
        PersonSeed {
            id: SEED_PERSON_WERNER_ID,
            given: "Werner",
            family: "Schmidt",
            name_at_birth: "",
            nickname: "",
            gender: "male",
            birth_date: ymd(1936, 5, 18),
            birth_place: "München",
            death_date: None,
            notes: "Retired engineer; lives in Bayern.",
            linked_user_id: None,
        },
        PersonSeed {
            id: SEED_PERSON_GRETA_ID,
            given: "Greta",
            family: "Schmidt",
            name_at_birth: "Hoffmann",
            nickname: "",
            gender: "female",
            birth_date: ymd(1940, 2, 9),
            birth_place: "Augsburg",
            death_date: None,
            notes: "Long-time librarian; family historian.",
            linked_user_id: None,
        },
        // -------------------------------------------------------------
        // G1 — Bauer line (a third grandparent couple, Lotte deceased).
        // -------------------------------------------------------------
        PersonSeed {
            id: SEED_PERSON_FRIEDRICH_ID,
            given: "Friedrich",
            family: "Bauer",
            name_at_birth: "",
            nickname: "Fritz",
            gender: "male",
            birth_date: ymd(1932, 4, 10),
            birth_place: "Stuttgart",
            death_date: None,
            notes: "Widower since 2018; spends summers in the Alps.",
            linked_user_id: None,
        },
        PersonSeed {
            id: SEED_PERSON_LOTTE_ID,
            given: "Lotte",
            family: "Bauer",
            name_at_birth: "Wagner",
            nickname: "",
            gender: "female",
            birth_date: ymd(1934, 9, 22),
            birth_place: "Stuttgart",
            death_date: Some(ymd(2018, 8, 15)),
            notes: "Pediatric nurse; passed in 2018 after a long illness.",
            linked_user_id: None,
        },
        // -------------------------------------------------------------
        // G2 Müller + an in-married ex (Klaus's first wife).
        // -------------------------------------------------------------
        PersonSeed {
            id: SEED_PERSON_KLAUS_ID,
            given: "Klaus",
            family: "Müller",
            name_at_birth: "",
            nickname: "",
            gender: "male",
            birth_date: ymd(1965, 4, 22),
            birth_place: "Hamburg",
            death_date: None,
            notes: "Owner of the seeded family; runs a small architecture studio.",
            linked_user_id: Some(SEED_ADMIN_USER_ID),
        },
        PersonSeed {
            id: SEED_PERSON_ANNA_ID,
            given: "Anna",
            family: "Müller",
            name_at_birth: "Schmidt",
            nickname: "Annie",
            gender: "female",
            birth_date: ymd(1968, 8, 11),
            birth_place: "München",
            death_date: None,
            notes: "Pediatrician; née Schmidt — took Müller after partnering with Klaus.",
            linked_user_id: Some(SEED_ALICE_USER_ID),
        },
        PersonSeed {
            id: SEED_PERSON_BRIGITTE_ID,
            given: "Brigitte",
            family: "Mayer",
            name_at_birth: "",
            nickname: "",
            gender: "female",
            birth_date: ymd(1968, 11, 30),
            birth_place: "Frankfurt",
            death_date: None,
            notes: "Klaus's first wife (married 1990, divorced 2000); mother of Felix.",
            linked_user_id: None,
        },
        // -------------------------------------------------------------
        // G2 Schmidt sister + her partner (same-sex civil_union).
        // -------------------------------------------------------------
        PersonSeed {
            id: SEED_PERSON_SABINE_ID,
            given: "Sabine",
            family: "Schmidt",
            name_at_birth: "",
            nickname: "Sabi",
            gender: "female",
            birth_date: ymd(1970, 9, 14),
            birth_place: "München",
            death_date: None,
            notes: "Anna's younger sister; software architect, lives in Köln with Julia.",
            linked_user_id: None,
        },
        PersonSeed {
            id: SEED_PERSON_JULIA_ID,
            given: "Julia",
            family: "Weber",
            name_at_birth: "",
            nickname: "",
            gender: "female",
            birth_date: ymd(1972, 3, 5),
            birth_place: "Köln",
            death_date: None,
            notes: "Sabine's civil-union partner; veterinarian.",
            linked_user_id: None,
        },
        // -------------------------------------------------------------
        // G2 Bauer son (single parent to Tom).
        // -------------------------------------------------------------
        PersonSeed {
            id: SEED_PERSON_MARKUS_ID,
            given: "Markus",
            family: "Bauer",
            name_at_birth: "",
            nickname: "",
            gender: "male",
            birth_date: ymd(1967, 2, 18),
            birth_place: "Stuttgart",
            death_date: None,
            notes: "Friedrich + Lotte's son; raising Tom on his own.",
            linked_user_id: None,
        },
        // -------------------------------------------------------------
        // G3 Müller — Lina, Max, Mia (Klaus + Anna) and Felix (Klaus + Brigitte).
        // -------------------------------------------------------------
        PersonSeed {
            id: SEED_PERSON_FELIX_ID,
            given: "Felix",
            family: "Müller",
            name_at_birth: "",
            nickname: "",
            gender: "male",
            birth_date: ymd(1992, 7, 8),
            birth_place: "Hamburg",
            death_date: None,
            notes: "Klaus + Brigitte's son; half-brother to Lina, Max, Mia. \
                    Step-mother Anna raised him after the 2000 divorce.",
            linked_user_id: None,
        },
        PersonSeed {
            id: SEED_PERSON_LINA_ID,
            given: "Lina",
            family: "Müller",
            name_at_birth: "",
            nickname: "Lini",
            gender: "female",
            birth_date: ymd(1995, 12, 3),
            birth_place: "Berlin",
            death_date: None,
            notes: "G3 — software developer in Berlin; mother of Emma and Noah.",
            linked_user_id: Some(SEED_BOB_USER_ID),
        },
        PersonSeed {
            id: SEED_PERSON_MAX_ID,
            given: "Max",
            family: "Müller",
            name_at_birth: "",
            nickname: "",
            gender: "male",
            birth_date: ymd(1998, 4, 17),
            birth_place: "Berlin",
            death_date: None,
            notes: "G3 — university student studying chemistry.",
            linked_user_id: None,
        },
        PersonSeed {
            id: SEED_PERSON_MIA_ID,
            given: "Mia",
            family: "Müller",
            name_at_birth: "",
            nickname: "",
            gender: "female",
            birth_date: ymd(2001, 9, 15),
            birth_place: "Hamburg",
            death_date: None,
            notes: "Youngest Klaus + Anna child; high-school graduate.",
            linked_user_id: None,
        },
        // -------------------------------------------------------------
        // G3 Sabine + Julia's adopted daughter, and Markus's son.
        // -------------------------------------------------------------
        PersonSeed {
            id: SEED_PERSON_LENA_ID,
            given: "Lena",
            family: "Weber-Schmidt",
            name_at_birth: "",
            nickname: "",
            gender: "female",
            birth_date: ymd(2005, 4, 22),
            birth_place: "Köln",
            death_date: None,
            notes: "Adopted by Sabine + Julia in 2007; both parents registered as adoptive.",
            linked_user_id: None,
        },
        PersonSeed {
            id: SEED_PERSON_TOM_ID,
            given: "Tom",
            family: "Bauer",
            name_at_birth: "",
            nickname: "",
            gender: "male",
            birth_date: ymd(1996, 6, 30),
            birth_place: "Stuttgart",
            death_date: None,
            notes: "Markus's son; raised by a single parent.",
            linked_user_id: None,
        },
        // -------------------------------------------------------------
        // G4 — Lina's children (no partnership row, single mother).
        // -------------------------------------------------------------
        PersonSeed {
            id: SEED_PERSON_EMMA_ID,
            given: "Emma",
            family: "Müller",
            name_at_birth: "",
            nickname: "",
            gender: "female",
            birth_date: ymd(2020, 5, 10),
            birth_place: "Berlin",
            death_date: None,
            notes: "Lina's daughter; G4.",
            linked_user_id: None,
        },
        PersonSeed {
            id: SEED_PERSON_NOAH_ID,
            given: "Noah",
            family: "Müller",
            name_at_birth: "",
            nickname: "",
            gender: "male",
            birth_date: ymd(2022, 11, 3),
            birth_place: "Berlin",
            death_date: None,
            notes: "Lina's son; G4, born after Emma.",
            linked_user_id: None,
        },
    ];

    for p in rows {
        sqlx::query(
            "INSERT INTO persons \
                 (id, family_id, given_name, family_name, name_at_birth, nickname, gender, \
                  birth_date, birth_place, death_date, notes, linked_user_id) \
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
        .bind(SEED_FAMILY_ID)
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
    }
    Ok(())
}
