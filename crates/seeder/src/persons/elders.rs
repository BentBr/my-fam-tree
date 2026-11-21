//! Seed persons: G1 grandparent couples (Müller / Schmidt / Bauer) and G2.

use super::{PersonSeed, ymd};
use crate::ids::{
    SEED_ADMIN_USER_ID, SEED_ALICE_USER_ID, SEED_PERSON_ANNA_ID, SEED_PERSON_BRIGITTE_ID,
    SEED_PERSON_FRIEDRICH_ID, SEED_PERSON_GRETA_ID, SEED_PERSON_HANNELORE_ID, SEED_PERSON_JULIA_ID,
    SEED_PERSON_KARIN_ID, SEED_PERSON_KLAUS_ID, SEED_PERSON_LOTTE_ID, SEED_PERSON_MARKUS_ID,
    SEED_PERSON_OTTO_ID, SEED_PERSON_SABINE_ID, SEED_PERSON_WERNER_ID, SEED_PERSON_YUKI_ID,
};

pub(super) const ROWS: [PersonSeed; 14] = [
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
];
