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
//!
//! Phase 2c added the contact columns (email/phone/postal address). Three
//! rows — Klaus, Anna and Hannelore — carry realistic German contact data so
//! the FE drawer's Contact section has something visible end-to-end. Every
//! other row uses [`EMPTY_CONTACT`], matching the production "Add Person"
//! flow where contact info is opt-in.

use chrono::NaiveDate;
use sqlx::PgPool;
use uuid::Uuid;

use crate::ids::{
    SEED_ADMIN_USER_ID, SEED_ALICE_USER_ID, SEED_BOB_USER_ID, SEED_FAMILY_ID, SEED_PERSON_ANNA_ID,
    SEED_PERSON_BRIGITTE_ID, SEED_PERSON_EMMA_ID, SEED_PERSON_FELIX_ID, SEED_PERSON_FRIEDRICH_ID,
    SEED_PERSON_GRETA_ID, SEED_PERSON_HANNELORE_ID, SEED_PERSON_JULIA_ID, SEED_PERSON_KARIN_ID,
    SEED_PERSON_KLAUS_ID, SEED_PERSON_LENA_ID, SEED_PERSON_LINA_ID, SEED_PERSON_LOTTE_ID,
    SEED_PERSON_MARKUS_ID, SEED_PERSON_MAX_ID, SEED_PERSON_MIA_ID, SEED_PERSON_NOAH_ID,
    SEED_PERSON_OTTO_ID, SEED_PERSON_SABINE_ID, SEED_PERSON_TOM_ID, SEED_PERSON_WERNER_ID,
    SEED_PERSON_YUKI_ID,
};

/// Contact-info bundle. Split out from `PersonSeed` so unenriched rows can
/// reference a single [`EMPTY_CONTACT`] const instead of repeating seven
/// empty-string fields each — keeps the seed table inside the 500-line cap.
struct Contact {
    /// Contact email. For rows with `linked_user_id` set the API rewrites
    /// this on every write to mirror `users.email`; we seed the matching
    /// value so the first `GET /persons` after `cargo run --bin seeder`
    /// already shows the synced address.
    email: &'static str,
    phone: &'static str,
    street: &'static str,
    house_number: &'static str,
    zip: &'static str,
    city: &'static str,
    country: &'static str,
}

const EMPTY_CONTACT: Contact =
    Contact { email: "", phone: "", street: "", house_number: "", zip: "", city: "", country: "" };

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
    contact: Contact,
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
    let rows: [PersonSeed; 22] = [
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
            contact: EMPTY_CONTACT,
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
            contact: Contact {
                email: "hannelore.mueller@example.de",
                phone: "+49 40 5550199",
                street: "Eppendorfer Landstraße",
                house_number: "47",
                zip: "20249",
                city: "Hamburg",
                country: "Deutschland",
            },
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
            contact: EMPTY_CONTACT,
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
            contact: EMPTY_CONTACT,
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
            contact: EMPTY_CONTACT,
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
            contact: EMPTY_CONTACT,
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
            // `email` is rewritten by the API to mirror `users.email`
            // whenever `linked_user_id` is set; seeding it as
            // `admin@example.com` so the first /persons read already matches
            // the synced value.
            contact: Contact {
                email: "admin@example.com",
                phone: "+49 40 5550101",
                street: "Mittelweg",
                house_number: "12",
                zip: "20148",
                city: "Hamburg",
                country: "Deutschland",
            },
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
            // Same household as Klaus; email synced from alice@example.com.
            contact: Contact {
                email: "alice@example.com",
                phone: "+49 40 5550102",
                street: "Mittelweg",
                house_number: "12",
                zip: "20148",
                city: "Hamburg",
                country: "Deutschland",
            },
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
            contact: EMPTY_CONTACT,
            linked_user_id: None,
        },
        // Klaus's earliest partner — separated in 1989 before the Brigitte
        // marriage. No children together; serves as the "second old" partner
        // edge case in the multi-spouse chain.
        PersonSeed {
            id: SEED_PERSON_KARIN_ID,
            given: "Karin",
            family: "Hoffmann",
            name_at_birth: "",
            nickname: "",
            gender: "female",
            birth_date: ymd(1966, 5, 7),
            birth_place: "Bremen",
            death_date: None,
            notes: "Klaus's first long-term partner (separated 1989); no children together.",
            contact: EMPTY_CONTACT,
            linked_user_id: None,
        },
        // Klaus's concurrent open partner alongside Anna — covers the
        // "polyamorous / multi-active partnership" edge case so the chain
        // renders as [Karin, Brigitte, Klaus, Anna, Yuki].
        PersonSeed {
            id: SEED_PERSON_YUKI_ID,
            given: "Yuki",
            family: "Tanaka",
            name_at_birth: "",
            nickname: "",
            gender: "female",
            birth_date: ymd(1975, 10, 19),
            birth_place: "Berlin",
            death_date: None,
            notes: "Klaus's second open partner; concurrent civil_union since 2015.",
            contact: EMPTY_CONTACT,
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
            contact: EMPTY_CONTACT,
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
            contact: EMPTY_CONTACT,
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
            contact: EMPTY_CONTACT,
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
            contact: EMPTY_CONTACT,
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
            // bob@example.com synced via linked_user_id.
            contact: Contact { email: "bob@example.com", ..EMPTY_CONTACT },
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
            contact: EMPTY_CONTACT,
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
            contact: EMPTY_CONTACT,
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
            contact: EMPTY_CONTACT,
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
            contact: EMPTY_CONTACT,
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
            contact: EMPTY_CONTACT,
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
            contact: EMPTY_CONTACT,
            linked_user_id: None,
        },
    ];

    for p in rows {
        sqlx::query(
            "INSERT INTO persons \
                 (id, family_id, given_name, family_name, name_at_birth, nickname, gender, \
                  birth_date, birth_place, death_date, notes, \
                  email, phone, street, house_number, zip, city, country, \
                  linked_user_id) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, \
                     $12, $13, $14, $15, $16, $17, $18, $19) \
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
                 email = EXCLUDED.email, \
                 phone = EXCLUDED.phone, \
                 street = EXCLUDED.street, \
                 house_number = EXCLUDED.house_number, \
                 zip = EXCLUDED.zip, \
                 city = EXCLUDED.city, \
                 country = EXCLUDED.country, \
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
        .bind(p.contact.email)
        .bind(p.contact.phone)
        .bind(p.contact.street)
        .bind(p.contact.house_number)
        .bind(p.contact.zip)
        .bind(p.contact.city)
        .bind(p.contact.country)
        .bind(p.linked_user_id)
        .execute(pool)
        .await?;
    }
    Ok(())
}
