//! Seed `parent_links` rows: one `(child, parent, kind)` per edge.

use super::ParentLinkSeed;
use crate::ids::{
    SEED_PERSON_ANNA_ID, SEED_PERSON_BRIGITTE_ID, SEED_PERSON_EMMA_ID, SEED_PERSON_FELIX_ID,
    SEED_PERSON_FLK_EDGAR_ID, SEED_PERSON_FLK_GISELA_ID, SEED_PERSON_FLK_ROLAND_ID,
    SEED_PERSON_FRIEDRICH_ID, SEED_PERSON_GRETA_ID, SEED_PERSON_HANNELORE_ID, SEED_PERSON_JULIA_ID,
    SEED_PERSON_K_ANNELIESE_ID, SEED_PERSON_K_GRETA_ID, SEED_PERSON_K_HELGA_ID,
    SEED_PERSON_K_HUBERT_ID, SEED_PERSON_K_LARS_ID, SEED_PERSON_K_MARIE_ID, SEED_PERSON_K_MIA_ID,
    SEED_PERSON_K_REINHARDT_ID, SEED_PERSON_K_SARA_ID, SEED_PERSON_K_TIM_ID, SEED_PERSON_KLAUS_ID,
    SEED_PERSON_LENA_ID, SEED_PERSON_LINA_ID, SEED_PERSON_LOTTE_ID, SEED_PERSON_MARKUS_ID,
    SEED_PERSON_MAX_ID, SEED_PERSON_MIA_ID, SEED_PERSON_NOAH_ID, SEED_PERSON_OTTO_ID,
    SEED_PERSON_SABINE_ID, SEED_PERSON_STB_CARLA_ID, SEED_PERSON_STB_FELIX_ID,
    SEED_PERSON_STB_HARTMUT_ID, SEED_PERSON_STB_LUKAS_ID, SEED_PERSON_STB_MARGARETE_ID,
    SEED_PERSON_STB_NINA_ID, SEED_PERSON_STB_STEFAN_ID, SEED_PERSON_TOM_ID, SEED_PERSON_WERNER_ID,
};

pub(super) const PARENT_LINKS: [ParentLinkSeed; 44] = [
    // Klaus + Anna lineage.
    ParentLinkSeed { child: SEED_PERSON_KLAUS_ID, parent: SEED_PERSON_OTTO_ID, kind: "biological" },
    ParentLinkSeed {
        child: SEED_PERSON_KLAUS_ID,
        parent: SEED_PERSON_HANNELORE_ID,
        kind: "biological",
    },
    ParentLinkSeed {
        child: SEED_PERSON_ANNA_ID,
        parent: SEED_PERSON_WERNER_ID,
        kind: "biological",
    },
    ParentLinkSeed { child: SEED_PERSON_ANNA_ID, parent: SEED_PERSON_GRETA_ID, kind: "biological" },
    // Sabine (Anna's bio sister).
    ParentLinkSeed {
        child: SEED_PERSON_SABINE_ID,
        parent: SEED_PERSON_WERNER_ID,
        kind: "biological",
    },
    ParentLinkSeed {
        child: SEED_PERSON_SABINE_ID,
        parent: SEED_PERSON_GRETA_ID,
        kind: "biological",
    },
    // Markus (Friedrich + Lotte's son).
    ParentLinkSeed {
        child: SEED_PERSON_MARKUS_ID,
        parent: SEED_PERSON_FRIEDRICH_ID,
        kind: "biological",
    },
    ParentLinkSeed {
        child: SEED_PERSON_MARKUS_ID,
        parent: SEED_PERSON_LOTTE_ID,
        kind: "biological",
    },
    // Felix — half-sibling. Bio parents Klaus + Brigitte; Anna is
    // step-mother (the post-divorce-remarriage configuration).
    ParentLinkSeed {
        child: SEED_PERSON_FELIX_ID,
        parent: SEED_PERSON_KLAUS_ID,
        kind: "biological",
    },
    ParentLinkSeed {
        child: SEED_PERSON_FELIX_ID,
        parent: SEED_PERSON_BRIGITTE_ID,
        kind: "biological",
    },
    ParentLinkSeed { child: SEED_PERSON_FELIX_ID, parent: SEED_PERSON_ANNA_ID, kind: "step" },
    // Klaus + Anna's biological children: Lina, Max, Mia.
    ParentLinkSeed { child: SEED_PERSON_LINA_ID, parent: SEED_PERSON_KLAUS_ID, kind: "biological" },
    ParentLinkSeed { child: SEED_PERSON_LINA_ID, parent: SEED_PERSON_ANNA_ID, kind: "biological" },
    ParentLinkSeed { child: SEED_PERSON_MAX_ID, parent: SEED_PERSON_KLAUS_ID, kind: "biological" },
    ParentLinkSeed { child: SEED_PERSON_MAX_ID, parent: SEED_PERSON_ANNA_ID, kind: "biological" },
    ParentLinkSeed { child: SEED_PERSON_MIA_ID, parent: SEED_PERSON_KLAUS_ID, kind: "biological" },
    ParentLinkSeed { child: SEED_PERSON_MIA_ID, parent: SEED_PERSON_ANNA_ID, kind: "biological" },
    // Lena adopted by both Sabine and Julia.
    ParentLinkSeed { child: SEED_PERSON_LENA_ID, parent: SEED_PERSON_SABINE_ID, kind: "adoptive" },
    ParentLinkSeed { child: SEED_PERSON_LENA_ID, parent: SEED_PERSON_JULIA_ID, kind: "adoptive" },
    // Tom — single-parent (Markus only).
    ParentLinkSeed { child: SEED_PERSON_TOM_ID, parent: SEED_PERSON_MARKUS_ID, kind: "biological" },
    // G4 grandchildren — Lina is sole listed parent (single mother).
    ParentLinkSeed { child: SEED_PERSON_EMMA_ID, parent: SEED_PERSON_LINA_ID, kind: "biological" },
    ParentLinkSeed { child: SEED_PERSON_NOAH_ID, parent: SEED_PERSON_LINA_ID, kind: "biological" },
    // Krause subtree parent links (fixtures for layout edge cases).
    //   K_Greta     → K_Hubert (one mother → one son)
    //   K_Anneliese → K_Reinhardt (one mother → one son)
    //   K_Hubert + K_Sara → K_Mia
    //   K_Reinhardt + K_Helga → K_Lars, K_Marie, K_Tim
    ParentLinkSeed {
        child: SEED_PERSON_K_HUBERT_ID,
        parent: SEED_PERSON_K_GRETA_ID,
        kind: "biological",
    },
    ParentLinkSeed {
        child: SEED_PERSON_K_REINHARDT_ID,
        parent: SEED_PERSON_K_ANNELIESE_ID,
        kind: "biological",
    },
    ParentLinkSeed {
        child: SEED_PERSON_K_MIA_ID,
        parent: SEED_PERSON_K_HUBERT_ID,
        kind: "biological",
    },
    ParentLinkSeed {
        child: SEED_PERSON_K_MIA_ID,
        parent: SEED_PERSON_K_SARA_ID,
        kind: "biological",
    },
    ParentLinkSeed {
        child: SEED_PERSON_K_LARS_ID,
        parent: SEED_PERSON_K_REINHARDT_ID,
        kind: "biological",
    },
    ParentLinkSeed {
        child: SEED_PERSON_K_LARS_ID,
        parent: SEED_PERSON_K_HELGA_ID,
        kind: "biological",
    },
    ParentLinkSeed {
        child: SEED_PERSON_K_MARIE_ID,
        parent: SEED_PERSON_K_REINHARDT_ID,
        kind: "biological",
    },
    ParentLinkSeed {
        child: SEED_PERSON_K_MARIE_ID,
        parent: SEED_PERSON_K_HELGA_ID,
        kind: "biological",
    },
    ParentLinkSeed {
        child: SEED_PERSON_K_TIM_ID,
        parent: SEED_PERSON_K_REINHARDT_ID,
        kind: "biological",
    },
    ParentLinkSeed {
        child: SEED_PERSON_K_TIM_ID,
        parent: SEED_PERSON_K_HELGA_ID,
        kind: "biological",
    },
    // Steinbach: 6 children of Hartmut + Margarete.
    ParentLinkSeed {
        child: SEED_PERSON_STB_LUKAS_ID,
        parent: SEED_PERSON_STB_HARTMUT_ID,
        kind: "biological",
    },
    ParentLinkSeed {
        child: SEED_PERSON_STB_LUKAS_ID,
        parent: SEED_PERSON_STB_MARGARETE_ID,
        kind: "biological",
    },
    ParentLinkSeed {
        child: SEED_PERSON_STB_CARLA_ID,
        parent: SEED_PERSON_STB_HARTMUT_ID,
        kind: "biological",
    },
    ParentLinkSeed {
        child: SEED_PERSON_STB_CARLA_ID,
        parent: SEED_PERSON_STB_MARGARETE_ID,
        kind: "biological",
    },
    ParentLinkSeed {
        child: SEED_PERSON_STB_FELIX_ID,
        parent: SEED_PERSON_STB_HARTMUT_ID,
        kind: "biological",
    },
    ParentLinkSeed {
        child: SEED_PERSON_STB_FELIX_ID,
        parent: SEED_PERSON_STB_MARGARETE_ID,
        kind: "biological",
    },
    ParentLinkSeed {
        child: SEED_PERSON_STB_STEFAN_ID,
        parent: SEED_PERSON_STB_HARTMUT_ID,
        kind: "biological",
    },
    ParentLinkSeed {
        child: SEED_PERSON_STB_STEFAN_ID,
        parent: SEED_PERSON_STB_MARGARETE_ID,
        kind: "biological",
    },
    ParentLinkSeed {
        child: SEED_PERSON_STB_NINA_ID,
        parent: SEED_PERSON_STB_HARTMUT_ID,
        kind: "biological",
    },
    ParentLinkSeed {
        child: SEED_PERSON_STB_NINA_ID,
        parent: SEED_PERSON_STB_MARGARETE_ID,
        kind: "biological",
    },
    // Falke: Roland is Edgar+Gisela's child. No grandchildren in
    // the seed — earlier Liyah / Alina rows were removed (they
    // rendered as orphan-looking G3 cards with no partner link).
    ParentLinkSeed {
        child: SEED_PERSON_FLK_ROLAND_ID,
        parent: SEED_PERSON_FLK_EDGAR_ID,
        kind: "biological",
    },
    ParentLinkSeed {
        child: SEED_PERSON_FLK_ROLAND_ID,
        parent: SEED_PERSON_FLK_GISELA_ID,
        kind: "biological",
    },
];
