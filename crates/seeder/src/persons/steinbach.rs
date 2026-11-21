//! Seed persons: Steinbach subtree (sibling-by-age layout fixture).

use super::{PersonSeed, ymd};
use crate::ids::{
    SEED_PERSON_STB_BEATE_ID, SEED_PERSON_STB_CARLA_ID, SEED_PERSON_STB_FELIX_ID,
    SEED_PERSON_STB_HARTMUT_ID, SEED_PERSON_STB_LUKAS_ID, SEED_PERSON_STB_MARGARETE_ID,
    SEED_PERSON_STB_NINA_ID, SEED_PERSON_STB_STEFAN_ID, SEED_PERSON_STB_TOBIAS_ID,
};

pub(super) const ROWS: [PersonSeed; 9] = [
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
];
