//! Seed `partnerships` rows: open / divorced / widowed / same-sex cases.

use uuid::Uuid;

use super::{PartnershipSeed, order_pair, ymd};
use crate::ids::{
    SEED_PARTNERSHIP_FLK_EDGAR_GISELA_ID, SEED_PARTNERSHIP_FRIEDRICH_LOTTE_ID,
    SEED_PARTNERSHIP_HEINZ_URSULA_ID, SEED_PARTNERSHIP_K_HUBERT_SARA_ID,
    SEED_PARTNERSHIP_K_REINHARDT_HELGA_ID, SEED_PARTNERSHIP_K_TIM_MIA_ID,
    SEED_PARTNERSHIP_KLAUS_ANNA_ID, SEED_PARTNERSHIP_KLAUS_BRIGITTE_ID,
    SEED_PARTNERSHIP_KLAUS_KARIN_ID, SEED_PARTNERSHIP_KLAUS_YUKI_ID,
    SEED_PARTNERSHIP_LARS_METTE_ID, SEED_PARTNERSHIP_OTTO_HANNELORE_ID,
    SEED_PARTNERSHIP_SABINE_JULIA_ID, SEED_PARTNERSHIP_STB_CARLA_TOBIAS_ID,
    SEED_PARTNERSHIP_STB_FELIX_BEATE_ID, SEED_PARTNERSHIP_STB_HARTMUT_MARGARETE_ID,
    SEED_PARTNERSHIP_SVEN_MAREN_ID, SEED_PARTNERSHIP_WERNER_GRETA_ID, SEED_PERSON_ANNA_ID,
    SEED_PERSON_BRIGITTE_ID, SEED_PERSON_FLK_EDGAR_ID, SEED_PERSON_FLK_GISELA_ID,
    SEED_PERSON_FRIEDRICH_ID, SEED_PERSON_GRETA_ID, SEED_PERSON_HANNELORE_ID, SEED_PERSON_HEINZ_ID,
    SEED_PERSON_JULIA_ID, SEED_PERSON_K_HELGA_ID, SEED_PERSON_K_HUBERT_ID, SEED_PERSON_K_MIA_ID,
    SEED_PERSON_K_REINHARDT_ID, SEED_PERSON_K_SARA_ID, SEED_PERSON_K_TIM_ID, SEED_PERSON_KARIN_ID,
    SEED_PERSON_KLAUS_ID, SEED_PERSON_LARS_ID, SEED_PERSON_LOTTE_ID, SEED_PERSON_MAREN_ID,
    SEED_PERSON_METTE_ID, SEED_PERSON_OTTO_ID, SEED_PERSON_SABINE_ID, SEED_PERSON_STB_BEATE_ID,
    SEED_PERSON_STB_CARLA_ID, SEED_PERSON_STB_FELIX_ID, SEED_PERSON_STB_HARTMUT_ID,
    SEED_PERSON_STB_MARGARETE_ID, SEED_PERSON_STB_TOBIAS_ID, SEED_PERSON_SVEN_ID,
    SEED_PERSON_URSULA_ID, SEED_PERSON_WERNER_ID, SEED_PERSON_YUKI_ID,
};

// Pre-order each partner pair so the `partner_a_id < partner_b_id`
// CHECK constraint is satisfied byte-for-byte in seeded data.
const OTTO_HANNELORE: (Uuid, Uuid) = order_pair(SEED_PERSON_OTTO_ID, SEED_PERSON_HANNELORE_ID);
const WERNER_GRETA: (Uuid, Uuid) = order_pair(SEED_PERSON_WERNER_ID, SEED_PERSON_GRETA_ID);
const KLAUS_ANNA: (Uuid, Uuid) = order_pair(SEED_PERSON_KLAUS_ID, SEED_PERSON_ANNA_ID);
const FRIEDRICH_LOTTE: (Uuid, Uuid) = order_pair(SEED_PERSON_FRIEDRICH_ID, SEED_PERSON_LOTTE_ID);
const KLAUS_BRIGITTE: (Uuid, Uuid) = order_pair(SEED_PERSON_KLAUS_ID, SEED_PERSON_BRIGITTE_ID);
const SABINE_JULIA: (Uuid, Uuid) = order_pair(SEED_PERSON_SABINE_ID, SEED_PERSON_JULIA_ID);
const KLAUS_KARIN: (Uuid, Uuid) = order_pair(SEED_PERSON_KLAUS_ID, SEED_PERSON_KARIN_ID);
const KLAUS_YUKI: (Uuid, Uuid) = order_pair(SEED_PERSON_KLAUS_ID, SEED_PERSON_YUKI_ID);
const SVEN_MAREN: (Uuid, Uuid) = order_pair(SEED_PERSON_SVEN_ID, SEED_PERSON_MAREN_ID);
const HEINZ_URSULA: (Uuid, Uuid) = order_pair(SEED_PERSON_HEINZ_ID, SEED_PERSON_URSULA_ID);
const LARS_METTE: (Uuid, Uuid) = order_pair(SEED_PERSON_LARS_ID, SEED_PERSON_METTE_ID);
const K_HUBERT_SARA: (Uuid, Uuid) = order_pair(SEED_PERSON_K_HUBERT_ID, SEED_PERSON_K_SARA_ID);
const K_REINHARDT_HELGA: (Uuid, Uuid) =
    order_pair(SEED_PERSON_K_REINHARDT_ID, SEED_PERSON_K_HELGA_ID);
const K_TIM_MIA: (Uuid, Uuid) = order_pair(SEED_PERSON_K_TIM_ID, SEED_PERSON_K_MIA_ID);
const STB_HARTMUT_MARGARETE: (Uuid, Uuid) =
    order_pair(SEED_PERSON_STB_HARTMUT_ID, SEED_PERSON_STB_MARGARETE_ID);
const STB_CARLA_TOBIAS: (Uuid, Uuid) =
    order_pair(SEED_PERSON_STB_CARLA_ID, SEED_PERSON_STB_TOBIAS_ID);
const STB_FELIX_BEATE: (Uuid, Uuid) =
    order_pair(SEED_PERSON_STB_FELIX_ID, SEED_PERSON_STB_BEATE_ID);
const FLK_EDGAR_GISELA: (Uuid, Uuid) =
    order_pair(SEED_PERSON_FLK_EDGAR_ID, SEED_PERSON_FLK_GISELA_ID);

pub(super) const PARTNERSHIPS: [PartnershipSeed; 18] = [
    PartnershipSeed {
        id: SEED_PARTNERSHIP_OTTO_HANNELORE_ID,
        partner_a: OTTO_HANNELORE.0,
        partner_b: OTTO_HANNELORE.1,
        kind: "marriage",
        started_on: Some(ymd(1962, 6, 9)),
        ended_on: None,
        end_reason: None,
        note: "Married in Hamburg.",
    },
    PartnershipSeed {
        id: SEED_PARTNERSHIP_WERNER_GRETA_ID,
        partner_a: WERNER_GRETA.0,
        partner_b: WERNER_GRETA.1,
        kind: "marriage",
        started_on: Some(ymd(1964, 8, 22)),
        ended_on: None,
        end_reason: None,
        note: "Married in Augsburg.",
    },
    PartnershipSeed {
        id: SEED_PARTNERSHIP_KLAUS_ANNA_ID,
        partner_a: KLAUS_ANNA.0,
        partner_b: KLAUS_ANNA.1,
        kind: "civil_union",
        started_on: Some(ymd(2002, 5, 18)),
        ended_on: None,
        end_reason: None,
        note: "Klaus's second partnership; civil union after the 2000 divorce.",
    },
    // Widowed — Lotte died 2018, partnership closed by death.
    PartnershipSeed {
        id: SEED_PARTNERSHIP_FRIEDRICH_LOTTE_ID,
        partner_a: FRIEDRICH_LOTTE.0,
        partner_b: FRIEDRICH_LOTTE.1,
        kind: "marriage",
        started_on: Some(ymd(1960, 9, 3)),
        ended_on: Some(ymd(2018, 8, 15)),
        end_reason: Some("death"),
        note: "Married in Stuttgart; ended by Lotte's passing.",
    },
    // Divorced — Klaus + Brigitte, ended 2000.
    PartnershipSeed {
        id: SEED_PARTNERSHIP_KLAUS_BRIGITTE_ID,
        partner_a: KLAUS_BRIGITTE.0,
        partner_b: KLAUS_BRIGITTE.1,
        kind: "marriage",
        started_on: Some(ymd(1990, 4, 14)),
        ended_on: Some(ymd(2000, 6, 30)),
        end_reason: Some("divorce"),
        note: "Klaus's first marriage; ended in divorce after 10 years.",
    },
    // Same-sex civil union, ongoing.
    PartnershipSeed {
        id: SEED_PARTNERSHIP_SABINE_JULIA_ID,
        partner_a: SABINE_JULIA.0,
        partner_b: SABINE_JULIA.1,
        kind: "civil_union",
        started_on: Some(ymd(2003, 11, 1)),
        ended_on: None,
        end_reason: None,
        note: "Civil union in Köln; later adopted Lena together.",
    },
    // Klaus's earliest partnership — separated (not divorced) in 1989,
    // before the Klaus + Brigitte marriage. Adds a second ENDED row on
    // Klaus so the multi-partner chain renders [Karin, Brigitte, Klaus].
    PartnershipSeed {
        id: SEED_PARTNERSHIP_KLAUS_KARIN_ID,
        partner_a: KLAUS_KARIN.0,
        partner_b: KLAUS_KARIN.1,
        kind: "partnership",
        started_on: Some(ymd(1987, 3, 5)),
        ended_on: Some(ymd(1989, 11, 20)),
        end_reason: Some("separation"),
        note: "Klaus's first long-term partnership; separated after two years.",
    },
    // Klaus's concurrent open partner alongside Anna — adds a second
    // OPEN row on Klaus so the chain extends to […, Klaus, Anna, Yuki].
    PartnershipSeed {
        id: SEED_PARTNERSHIP_KLAUS_YUKI_ID,
        partner_a: KLAUS_YUKI.0,
        partner_b: KLAUS_YUKI.1,
        kind: "civil_union",
        started_on: Some(ymd(2015, 7, 4)),
        ended_on: None,
        end_reason: None,
        note: "Klaus's concurrent open partner since 2015.",
    },
    // Standalone active-marriage demo couple (no shared children
    // in the seed). Surfaces the gold interlocking-rings glyph on
    // its own row of the tree canvas, isolated from the Müller
    // graph context.
    PartnershipSeed {
        id: SEED_PARTNERSHIP_SVEN_MAREN_ID,
        partner_a: SVEN_MAREN.0,
        partner_b: SVEN_MAREN.1,
        kind: "marriage",
        started_on: Some(ymd(2007, 8, 18)),
        ended_on: None,
        end_reason: None,
        note: "Active marriage demo (Sven + Maren Hoffmann).",
    },
    // Standalone divorced-marriage demo couple. Same isolation
    // intent as Sven + Maren above, but ended_on is set so the
    // canvas shows the greyed-out treatment for the line + glyph.
    PartnershipSeed {
        id: SEED_PARTNERSHIP_HEINZ_URSULA_ID,
        partner_a: HEINZ_URSULA.0,
        partner_b: HEINZ_URSULA.1,
        kind: "marriage",
        started_on: Some(ymd(1995, 5, 27)),
        ended_on: Some(ymd(2010, 11, 12)),
        end_reason: Some("divorce"),
        note: "Divorced-marriage demo (Heinz Becker + Ursula Schwarz).",
    },
    // Standalone broken non-marriage partnership. Surfaces the
    // greyed-out HEART glyph (the counterpart to Heinz + Ursula's
    // greyed-out rings) so the seed shows every glyph state
    // without graph context: active heart, active rings, ended
    // heart, ended rings.
    PartnershipSeed {
        id: SEED_PARTNERSHIP_LARS_METTE_ID,
        partner_a: LARS_METTE.0,
        partner_b: LARS_METTE.1,
        kind: "partnership",
        started_on: Some(ymd(2010, 4, 2)),
        ended_on: Some(ymd(2018, 9, 30)),
        end_reason: Some("separation"),
        note: "Separated non-marriage partnership demo (Lars + Mette).",
    },
    // Krause subtree partnerships (fixtures for layout edge cases).
    PartnershipSeed {
        id: SEED_PARTNERSHIP_K_HUBERT_SARA_ID,
        partner_a: K_HUBERT_SARA.0,
        partner_b: K_HUBERT_SARA.1,
        kind: "marriage",
        started_on: Some(ymd(1985, 6, 14)),
        ended_on: None,
        end_reason: None,
        note: "Krause subtree: Hubert + Sara; parents of Mia.",
    },
    PartnershipSeed {
        id: SEED_PARTNERSHIP_K_REINHARDT_HELGA_ID,
        partner_a: K_REINHARDT_HELGA.0,
        partner_b: K_REINHARDT_HELGA.1,
        kind: "marriage",
        started_on: Some(ymd(1983, 5, 21)),
        ended_on: None,
        end_reason: None,
        note: "Krause subtree: Reinhardt + Helga; parents of Lars, Marie, Tim.",
    },
    PartnershipSeed {
        id: SEED_PARTNERSHIP_K_TIM_MIA_ID,
        partner_a: K_TIM_MIA.0,
        partner_b: K_TIM_MIA.1,
        kind: "marriage",
        started_on: Some(ymd(2018, 9, 15)),
        ended_on: None,
        end_reason: None,
        note: "Krause subtree: Tim + Mia in-married couple; the layout \
               should put Tim LEFT (his parents on the left) and Mia \
               RIGHT (her parents on the right) to avoid crossing parent edges.",
    },
    // Steinbach: parents' marriage + two sibling couples.
    PartnershipSeed {
        id: SEED_PARTNERSHIP_STB_HARTMUT_MARGARETE_ID,
        partner_a: STB_HARTMUT_MARGARETE.0,
        partner_b: STB_HARTMUT_MARGARETE.1,
        kind: "marriage",
        started_on: Some(ymd(1968, 6, 15)),
        ended_on: None,
        end_reason: None,
        note: "Steinbach parents (Hartmut + Margarete).",
    },
    PartnershipSeed {
        id: SEED_PARTNERSHIP_STB_CARLA_TOBIAS_ID,
        partner_a: STB_CARLA_TOBIAS.0,
        partner_b: STB_CARLA_TOBIAS.1,
        kind: "marriage",
        started_on: Some(ymd(2001, 5, 12)),
        ended_on: None,
        end_reason: None,
        note: "Steinbach sub-tree: Carla + Tobias Brandt — Tobias's \
               smaller UUID makes him the block's leftId; his 1969 \
               birth date triggers the sibling-sort violation.",
    },
    PartnershipSeed {
        id: SEED_PARTNERSHIP_STB_FELIX_BEATE_ID,
        partner_a: STB_FELIX_BEATE.0,
        partner_b: STB_FELIX_BEATE.1,
        kind: "marriage",
        started_on: Some(ymd(2010, 9, 4)),
        ended_on: None,
        end_reason: None,
        note: "Steinbach sub-tree: Felix + Beate Voigt — Beate's \
               smaller UUID makes her the block's leftId; her 1985 \
               birth date pushes the couple to the end of the \
               sibling row.",
    },
    // Falke: parents' marriage (Roland is their only seeded child).
    PartnershipSeed {
        id: SEED_PARTNERSHIP_FLK_EDGAR_GISELA_ID,
        partner_a: FLK_EDGAR_GISELA.0,
        partner_b: FLK_EDGAR_GISELA.1,
        kind: "marriage",
        started_on: Some(ymd(1970, 6, 14)),
        ended_on: None,
        end_reason: None,
        note: "Falke parents (Edgar + Gisela); three-gen lineage chain demo.",
    },
];
