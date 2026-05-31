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

use chrono::NaiveDate;
use sqlx::PgPool;
use uuid::Uuid;

use crate::ids::{
    SEED_ADMIN_USER_ID, SEED_ALICE_USER_ID, SEED_BOB_USER_ID, SEED_FAMILY_ID, SEED_PERSON_ANNA_ID,
    SEED_PERSON_BRIGITTE_ID, SEED_PERSON_EMMA_ID, SEED_PERSON_FELIX_ID, SEED_PERSON_FLK_ALINA_ID,
    SEED_PERSON_FLK_DIRK_ID, SEED_PERSON_FLK_EDGAR_ID, SEED_PERSON_FLK_GISELA_ID,
    SEED_PERSON_FLK_LIYAH_ID, SEED_PERSON_FLK_ROLAND_ID, SEED_PERSON_FLK_SABINE_ID,
    SEED_PERSON_FRIEDRICH_ID, SEED_PERSON_GRETA_ID, SEED_PERSON_HANNELORE_ID, SEED_PERSON_HEINZ_ID,
    SEED_PERSON_JULIA_ID, SEED_PERSON_K_ANNELIESE_ID, SEED_PERSON_K_BERNHARD_ID,
    SEED_PERSON_K_GRETA_ID, SEED_PERSON_K_HELGA_ID, SEED_PERSON_K_HUBERT_ID, SEED_PERSON_K_LARS_ID,
    SEED_PERSON_K_MARIE_ID, SEED_PERSON_K_MIA_ID, SEED_PERSON_K_SARA_ID, SEED_PERSON_K_TIM_ID,
    SEED_PERSON_KARIN_ID, SEED_PERSON_KLAUS_ID, SEED_PERSON_LARS_ID, SEED_PERSON_LENA_ID,
    SEED_PERSON_LINA_ID, SEED_PERSON_LOTTE_ID, SEED_PERSON_MAREN_ID, SEED_PERSON_MARKUS_ID,
    SEED_PERSON_MAX_ID, SEED_PERSON_METTE_ID, SEED_PERSON_MIA_ID, SEED_PERSON_NOAH_ID,
    SEED_PERSON_OTTO_ID, SEED_PERSON_SABINE_ID, SEED_PERSON_STB_BEATE_ID, SEED_PERSON_STB_CARLA_ID,
    SEED_PERSON_STB_FELIX_ID, SEED_PERSON_STB_HARTMUT_ID, SEED_PERSON_STB_LUKAS_ID,
    SEED_PERSON_STB_MARGARETE_ID, SEED_PERSON_STB_NINA_ID, SEED_PERSON_STB_STEFAN_ID,
    SEED_PERSON_STB_TOBIAS_ID, SEED_PERSON_SVEN_ID, SEED_PERSON_TOM_ID, SEED_PERSON_URSULA_ID,
    SEED_PERSON_WERNER_ID, SEED_PERSON_WGN_HELMUT_ID, SEED_PERSON_WGN_INGRID_ID,
    SEED_PERSON_WGN_RENATE_ID, SEED_PERSON_YUKI_ID,
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
#[allow(clippy::too_many_lines, reason = "static table of 57 persons; splitting hurts readability")]
pub async fn seed_persons(pool: &PgPool) -> anyhow::Result<()> {
    let rows: [PersonSeed; 57] = [
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
        // -------------------------------------------------------------
        // Standalone couples — no parents / children in the seeded
        // family. They exist so the tree canvas has clean isolated
        // examples of the marriage glyph in both states:
        //   * Sven + Maren  — active marriage → gold interlocking rings
        //   * Heinz + Ursula — divorced marriage → greyed rings + line
        // -------------------------------------------------------------
        PersonSeed {
            id: SEED_PERSON_SVEN_ID,
            given: "Sven",
            family: "Hoffmann",
            name_at_birth: "",
            nickname: "",
            gender: "male",
            birth_date: ymd(1978, 2, 14),
            birth_place: "Lübeck",
            death_date: None,
            notes: "Active-marriage demo couple with Maren (no children in seed).",
            linked_user_id: None,
        },
        PersonSeed {
            id: SEED_PERSON_MAREN_ID,
            given: "Maren",
            family: "Hoffmann",
            name_at_birth: "Lindqvist",
            nickname: "",
            gender: "female",
            birth_date: ymd(1980, 6, 5),
            birth_place: "Kiel",
            death_date: None,
            notes: "Married Sven 2007; no children in seed.",
            linked_user_id: None,
        },
        PersonSeed {
            id: SEED_PERSON_HEINZ_ID,
            given: "Heinz",
            family: "Becker",
            name_at_birth: "",
            nickname: "",
            gender: "male",
            birth_date: ymd(1965, 10, 12),
            birth_place: "Dortmund",
            death_date: None,
            notes: "Divorced-marriage demo with Ursula; ended 2010.",
            linked_user_id: None,
        },
        PersonSeed {
            id: SEED_PERSON_URSULA_ID,
            given: "Ursula",
            family: "Schwarz",
            name_at_birth: "Becker",
            nickname: "Uschi",
            gender: "female",
            birth_date: ymd(1968, 3, 28),
            birth_place: "Essen",
            death_date: None,
            notes: "Divorced from Heinz 2010; kept maiden name Schwarz after divorce.",
            linked_user_id: None,
        },
        // Standalone separated-partnership demo: non-marriage, ended.
        // Shows the greyed-out HEART (not rings) on the canvas — the
        // counterpart to Heinz + Ursula's greyed-out rings.
        PersonSeed {
            id: SEED_PERSON_LARS_ID,
            given: "Lars",
            family: "Andersen",
            name_at_birth: "",
            nickname: "",
            gender: "male",
            birth_date: ymd(1982, 9, 4),
            birth_place: "Flensburg",
            death_date: None,
            notes: "Separated-partnership demo with Mette; ended 2018.",
            linked_user_id: None,
        },
        PersonSeed {
            id: SEED_PERSON_METTE_ID,
            given: "Mette",
            family: "Jensen",
            name_at_birth: "",
            nickname: "",
            gender: "female",
            birth_date: ymd(1984, 1, 19),
            birth_place: "Aarhus",
            death_date: None,
            notes: "Separated from Lars 2018 (non-marriage partnership).",
            linked_user_id: None,
        },
        // -------------------------------------------------------------
        // Steinbach subtree — sibling-by-age violation. The in-married
        // spouses (Tobias, Beate) have UUIDs LOWER than their blood
        // partners, so the layout's `members[0]` block-sort uses the
        // spouse's birth_date as the key. Tobias 1969 < Lukas 1972
        // pushes the Carla+Tobias couple LEFT of Lukas instead of
        // slotting right after him.
        // -------------------------------------------------------------
        PersonSeed {
            id: SEED_PERSON_STB_HARTMUT_ID,
            given: "Hartmut",
            family: "Steinbach",
            name_at_birth: "",
            nickname: "",
            gender: "male",
            birth_date: ymd(1940, 3, 12),
            birth_place: "Marburg",
            death_date: None,
            notes: "Steinbach patriarch; sibling-sort layout repro.",
            linked_user_id: None,
        },
        PersonSeed {
            id: SEED_PERSON_STB_MARGARETE_ID,
            given: "Margarete",
            family: "Steinbach",
            name_at_birth: "Reuther",
            nickname: "",
            gender: "female",
            birth_date: ymd(1948, 7, 25),
            birth_place: "Gießen",
            death_date: None,
            notes: "Steinbach matriarch.",
            linked_user_id: None,
        },
        PersonSeed {
            id: SEED_PERSON_STB_TOBIAS_ID,
            given: "Tobias",
            family: "Brandt",
            name_at_birth: "",
            nickname: "",
            gender: "male",
            // Born BEFORE the eldest blood-sibling Lukas — this is the
            // trigger: block-sort by `members[0]` (Tobias, smaller
            // UUID) uses 1969, pushing the Carla+Tobias couple to the
            // very leftmost of the sibling row.
            birth_date: ymd(1969, 8, 22),
            birth_place: "Kassel",
            death_date: None,
            notes: "In-married to Carla Steinbach. UUID < Carla's by design.",
            linked_user_id: None,
        },
        PersonSeed {
            id: SEED_PERSON_STB_CARLA_ID,
            given: "Carla",
            family: "Steinbach",
            name_at_birth: "",
            nickname: "",
            gender: "female",
            birth_date: ymd(1974, 11, 30),
            birth_place: "Marburg",
            death_date: None,
            notes: "Steinbach daughter; married Tobias Brandt.",
            linked_user_id: None,
        },
        PersonSeed {
            id: SEED_PERSON_STB_LUKAS_ID,
            given: "Lukas",
            family: "Steinbach",
            name_at_birth: "",
            nickname: "",
            gender: "male",
            birth_date: ymd(1972, 2, 8),
            birth_place: "Marburg",
            death_date: None,
            notes: "Steinbach son; eldest blood-sibling (single).",
            linked_user_id: None,
        },
        PersonSeed {
            id: SEED_PERSON_STB_BEATE_ID,
            given: "Beate",
            family: "Voigt",
            name_at_birth: "",
            nickname: "",
            gender: "female",
            // Born AFTER her blood-sibling-partner Felix; UUID is
            // still less than Felix's so the layout uses 1985 as the
            // block sort key, pushing Felix+Beate to the end of the
            // sibling row instead of slotting by Felix's 1977.
            birth_date: ymd(1985, 4, 10),
            birth_place: "Frankfurt",
            death_date: None,
            notes: "In-married to Felix Steinbach. UUID < Felix's by design.",
            linked_user_id: None,
        },
        PersonSeed {
            id: SEED_PERSON_STB_FELIX_ID,
            given: "Felix",
            family: "Steinbach",
            name_at_birth: "",
            nickname: "",
            gender: "male",
            birth_date: ymd(1977, 1, 15),
            birth_place: "Marburg",
            death_date: None,
            notes: "Steinbach son; married Beate Voigt.",
            linked_user_id: None,
        },
        PersonSeed {
            id: SEED_PERSON_STB_STEFAN_ID,
            given: "Stefan",
            family: "Steinbach",
            name_at_birth: "",
            nickname: "",
            gender: "male",
            birth_date: ymd(1978, 11, 4),
            birth_place: "Marburg",
            death_date: None,
            notes: "Steinbach son.",
            linked_user_id: None,
        },
        PersonSeed {
            id: SEED_PERSON_STB_NINA_ID,
            given: "Nina",
            family: "Steinbach",
            name_at_birth: "",
            nickname: "",
            gender: "female",
            birth_date: ymd(1983, 9, 19),
            birth_place: "Marburg",
            death_date: None,
            notes: "Steinbach daughter; youngest blood-sibling.",
            linked_user_id: None,
        },
        // -------------------------------------------------------------
        // Wagner subtree — anchor-in-middle violation. Helmut has TWO
        // concurrent OPEN partnerships (marriage + civil_union). The
        // layout's `threadComponent` only puts the anchor in the
        // middle when at least one partner is ended; with both open,
        // it currently builds [anchor, open1, open2] — Helmut leftmost.
        // -------------------------------------------------------------
        PersonSeed {
            id: SEED_PERSON_WGN_HELMUT_ID,
            given: "Helmut",
            family: "Wagner",
            name_at_birth: "",
            nickname: "",
            gender: "male",
            birth_date: ymd(1942, 5, 30),
            birth_place: "Stuttgart",
            death_date: None,
            notes: "Wagner anchor — has two concurrent open partnerships.",
            linked_user_id: None,
        },
        PersonSeed {
            id: SEED_PERSON_WGN_INGRID_ID,
            given: "Ingrid",
            family: "Wagner",
            name_at_birth: "Berger",
            nickname: "",
            gender: "female",
            birth_date: ymd(1945, 1, 14),
            birth_place: "Tübingen",
            death_date: None,
            notes: "Helmut's open marriage partner.",
            linked_user_id: None,
        },
        PersonSeed {
            id: SEED_PERSON_WGN_RENATE_ID,
            given: "Renate",
            family: "Wagner",
            name_at_birth: "Fuchs",
            nickname: "",
            gender: "female",
            birth_date: ymd(1948, 6, 3),
            birth_place: "Ulm",
            death_date: None,
            notes: "Helmut's open civil_union partner (concurrent with the marriage).",
            linked_user_id: None,
        },
        // -------------------------------------------------------------
        // Falke subtree — Lau-like multi-row crossing repro. Roland
        // Falke (only Falke with parents in the seed) marries into a
        // sibling row whose other members are partnerless roots. His
        // parents (Edgar + Gisela Falke) sit in the parent row but
        // the layout doesn't re-align them above Roland — they end
        // up rightmost while Roland is mid-row, so the parent-edge
        // crosses Sabine's column.
        // -------------------------------------------------------------
        PersonSeed {
            id: SEED_PERSON_FLK_EDGAR_ID,
            given: "Edgar",
            family: "Falke",
            name_at_birth: "",
            nickname: "",
            gender: "male",
            birth_date: ymd(1944, 10, 5),
            birth_place: "Hannover",
            death_date: None,
            notes: "Falke patriarch; multi-row barycenter layout repro.",
            linked_user_id: None,
        },
        PersonSeed {
            id: SEED_PERSON_FLK_GISELA_ID,
            given: "Gisela",
            family: "Falke",
            name_at_birth: "Weber",
            nickname: "",
            gender: "female",
            birth_date: ymd(1947, 2, 18),
            birth_place: "Bielefeld",
            death_date: None,
            notes: "Falke matriarch.",
            linked_user_id: None,
        },
        PersonSeed {
            id: SEED_PERSON_FLK_ROLAND_ID,
            given: "Roland",
            family: "Falke",
            name_at_birth: "",
            nickname: "",
            gender: "male",
            birth_date: ymd(1973, 6, 21),
            birth_place: "Hannover",
            death_date: None,
            notes: "Edgar + Gisela's son; the middle-row anchor whose parents drift right.",
            linked_user_id: None,
        },
        PersonSeed {
            id: SEED_PERSON_FLK_SABINE_ID,
            given: "Sabine",
            family: "Hahn",
            name_at_birth: "",
            nickname: "",
            gender: "female",
            birth_date: ymd(1972, 4, 18),
            birth_place: "Köln",
            death_date: None,
            notes: "Falke-row sibling (partnerless root); leftmost in the row.",
            linked_user_id: None,
        },
        PersonSeed {
            id: SEED_PERSON_FLK_DIRK_ID,
            given: "Dirk",
            family: "Sommer",
            name_at_birth: "",
            nickname: "",
            gender: "male",
            birth_date: ymd(1975, 9, 7),
            birth_place: "Bonn",
            death_date: None,
            notes: "Falke-row sibling (partnerless root); rightmost.",
            linked_user_id: None,
        },
        PersonSeed {
            id: SEED_PERSON_FLK_LIYAH_ID,
            given: "Liyah",
            family: "Falke",
            name_at_birth: "",
            nickname: "",
            gender: "female",
            birth_date: ymd(2017, 1, 23),
            birth_place: "Hannover",
            death_date: None,
            notes: "Roland's daughter.",
            linked_user_id: None,
        },
        PersonSeed {
            id: SEED_PERSON_FLK_ALINA_ID,
            given: "Alina",
            family: "Hahn",
            name_at_birth: "",
            nickname: "",
            gender: "female",
            birth_date: ymd(2000, 4, 5),
            birth_place: "Köln",
            death_date: None,
            notes: "Sabine Hahn's daughter (single-parent, no Falke partnership).",
            linked_user_id: None,
        },
        // -------------------------------------------------------------
        // Krause subtree — fixtures for the layout edge cases tracked
        // in fe/tests/components/tree/layout.crossing.test.ts and the
        // `upcoming-tree-layout-rules` memory. Two unpartnered
        // great-grandmothers at the top, two middle-row couples below
        // them, and one in-married couple at the bottom (Tim + Mia)
        // joining the two branches.
        // -------------------------------------------------------------
        PersonSeed {
            id: SEED_PERSON_K_GRETA_ID,
            given: "Greta",
            family: "Krause",
            name_at_birth: "Wendland",
            nickname: "",
            gender: "female",
            birth_date: ymd(1912, 3, 29),
            birth_place: "Königsberg",
            death_date: Some(ymd(2011, 3, 27)),
            notes: "Krause subtree, Edge-case fixture: top-row mother of Hubert.",
            linked_user_id: None,
        },
        PersonSeed {
            id: SEED_PERSON_K_ANNELIESE_ID,
            given: "Anneliese",
            family: "Krause",
            name_at_birth: "Schumann",
            nickname: "",
            gender: "female",
            birth_date: ymd(1921, 3, 25),
            birth_place: "Breslau",
            death_date: Some(ymd(2005, 3, 28)),
            notes: "Krause subtree, Edge-case fixture: top-row mother of Bernhard.",
            linked_user_id: None,
        },
        PersonSeed {
            id: SEED_PERSON_K_HUBERT_ID,
            given: "Hubert",
            family: "Krause",
            name_at_birth: "",
            nickname: "",
            gender: "male",
            birth_date: ymd(1947, 11, 7),
            birth_place: "Hamburg",
            death_date: None,
            notes: "Krause subtree: Greta's son; married Sara; father of Mia.",
            linked_user_id: None,
        },
        PersonSeed {
            id: SEED_PERSON_K_SARA_ID,
            given: "Sara",
            family: "Krause",
            name_at_birth: "Tanaka",
            nickname: "",
            gender: "female",
            birth_date: ymd(1956, 11, 22),
            birth_place: "Tokio",
            death_date: None,
            notes: "Krause subtree: married Hubert; mother of Mia.",
            linked_user_id: None,
        },
        PersonSeed {
            id: SEED_PERSON_K_BERNHARD_ID,
            given: "Bernhard",
            family: "Krause",
            name_at_birth: "",
            nickname: "",
            gender: "male",
            birth_date: ymd(1942, 11, 25),
            birth_place: "Köln",
            death_date: None,
            notes: "Krause subtree: Anneliese's son; married Helga; father of Lars, Marie, Tim.",
            linked_user_id: None,
        },
        PersonSeed {
            id: SEED_PERSON_K_HELGA_ID,
            given: "Helga",
            family: "Krause",
            name_at_birth: "Bornemann",
            nickname: "",
            gender: "female",
            birth_date: ymd(1958, 9, 12),
            birth_place: "Bremen",
            death_date: None,
            notes: "Krause subtree: married Bernhard; mother of Lars, Marie, Tim.",
            linked_user_id: None,
        },
        PersonSeed {
            id: SEED_PERSON_K_LARS_ID,
            given: "Lars",
            family: "Krause",
            name_at_birth: "",
            nickname: "",
            gender: "male",
            birth_date: ymd(1985, 1, 14),
            birth_place: "Köln",
            death_date: None,
            notes: "Krause subtree: ELDEST of three siblings — must stay LEFT on the row.",
            linked_user_id: None,
        },
        PersonSeed {
            id: SEED_PERSON_K_MARIE_ID,
            given: "Marie",
            family: "Krause",
            name_at_birth: "",
            nickname: "",
            gender: "female",
            birth_date: ymd(1987, 6, 15),
            birth_place: "Köln",
            death_date: None,
            notes: "Krause subtree: MIDDLE sibling — must stay between Lars and Tim.",
            linked_user_id: None,
        },
        PersonSeed {
            id: SEED_PERSON_K_TIM_ID,
            given: "Tim",
            family: "Krause",
            name_at_birth: "",
            nickname: "",
            gender: "male",
            birth_date: ymd(1989, 3, 22),
            birth_place: "Köln",
            death_date: None,
            notes: "Krause subtree: YOUNGEST sibling, married to Mia. \
                    Sibling-order regression target: Tim's couple block must \
                    not push Tim left of Marie.",
            linked_user_id: None,
        },
        PersonSeed {
            id: SEED_PERSON_K_MIA_ID,
            given: "Mia",
            family: "Krause",
            name_at_birth: "Krause",
            nickname: "",
            gender: "female",
            birth_date: ymd(1988, 5, 10),
            birth_place: "Hamburg",
            death_date: None,
            notes: "Krause subtree: Hubert + Sara's daughter, married Tim. \
                    In-married-couple-side regression target: Mia RIGHT, Tim LEFT.",
            linked_user_id: None,
        },
    ];

    for p in rows {
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
