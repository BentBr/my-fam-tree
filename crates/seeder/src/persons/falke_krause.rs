//! Seed persons: Falke lineage chain and Krause layout-edge-case subtree.

use super::{PersonSeed, ymd};
use crate::ids::{
    SEED_PERSON_FLK_EDGAR_ID, SEED_PERSON_FLK_GISELA_ID, SEED_PERSON_FLK_ROLAND_ID,
    SEED_PERSON_K_ANNELIESE_ID, SEED_PERSON_K_GRETA_ID, SEED_PERSON_K_HELGA_ID,
    SEED_PERSON_K_HUBERT_ID, SEED_PERSON_K_LARS_ID, SEED_PERSON_K_MARIE_ID, SEED_PERSON_K_MIA_ID,
    SEED_PERSON_K_REINHARDT_ID, SEED_PERSON_K_SARA_ID, SEED_PERSON_K_TIM_ID,
};

pub(super) const ROWS: [PersonSeed; 13] = [
    // -------------------------------------------------------------
    // Falke subtree — three-generation lineage chain. Edgar +
    // Gisela are G1; Roland is their only child (G2, no
    // descendants in the seed). Earlier revisions also seeded
    // floater siblings (Sabine Hahn, Dirk Sommer) and a Liyah /
    // Alina G3 layer; those rendered as orphan-looking cards in
    // the canvas (no partner, no grafted-in connection to the
    // rest of the family) and were removed.
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
        notes: "Falke patriarch; example of a clean three-gen lineage chain.",
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
        notes: "Edgar + Gisela's son.",
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
        notes: "Krause subtree, Edge-case fixture: top-row mother of Reinhardt.",
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
        id: SEED_PERSON_K_REINHARDT_ID,
        given: "Reinhardt",
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
        notes: "Krause subtree: married Reinhardt; mother of Lars, Marie, Tim.",
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
