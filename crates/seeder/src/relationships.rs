//! `parent_links` and `partnerships` for the dev seed.
//!
//! Both tables are upserted via `ON CONFLICT (…) DO UPDATE` so a re-seed
//! is idempotent. Partnership rows are keyed by hardcoded `id` (rather
//! than the partial-unique `(a, b, kind) WHERE ended_on IS NULL` index)
//! so closed/widowed rows can be re-seeded without inserting duplicates.

use chrono::NaiveDate;
use sqlx::PgPool;
use uuid::Uuid;

use crate::ids::{
    SEED_FAMILY_ID, SEED_PARTNERSHIP_FRIEDRICH_LOTTE_ID, SEED_PARTNERSHIP_HEINZ_URSULA_ID,
    SEED_PARTNERSHIP_KLAUS_ANNA_ID, SEED_PARTNERSHIP_KLAUS_BRIGITTE_ID,
    SEED_PARTNERSHIP_KLAUS_KARIN_ID, SEED_PARTNERSHIP_KLAUS_YUKI_ID,
    SEED_PARTNERSHIP_LARS_METTE_ID, SEED_PARTNERSHIP_OTTO_HANNELORE_ID,
    SEED_PARTNERSHIP_SABINE_JULIA_ID, SEED_PARTNERSHIP_SVEN_MAREN_ID,
    SEED_PARTNERSHIP_WERNER_GRETA_ID, SEED_PERSON_ANNA_ID, SEED_PERSON_BRIGITTE_ID,
    SEED_PERSON_EMMA_ID, SEED_PERSON_FELIX_ID, SEED_PERSON_FRIEDRICH_ID, SEED_PERSON_GRETA_ID,
    SEED_PERSON_HANNELORE_ID, SEED_PERSON_HEINZ_ID, SEED_PERSON_JULIA_ID, SEED_PERSON_KARIN_ID,
    SEED_PERSON_KLAUS_ID, SEED_PERSON_LARS_ID, SEED_PERSON_LENA_ID, SEED_PERSON_LINA_ID,
    SEED_PERSON_LOTTE_ID, SEED_PERSON_MAREN_ID, SEED_PERSON_MARKUS_ID, SEED_PERSON_MAX_ID,
    SEED_PERSON_METTE_ID, SEED_PERSON_MIA_ID, SEED_PERSON_NOAH_ID, SEED_PERSON_OTTO_ID,
    SEED_PERSON_SABINE_ID, SEED_PERSON_SVEN_ID, SEED_PERSON_TOM_ID, SEED_PERSON_URSULA_ID,
    SEED_PERSON_WERNER_ID, SEED_PERSON_YUKI_ID,
};

/// One `(child, parent)` row plus the relationship kind.
///
/// `biological` covers the canonical case. `step` / `adoptive` /
/// `legal` / `social` are for the edge-case fixtures (Felix's
/// stepmother, Lena's adoptive parents, etc.).
struct ParentLinkSeed {
    child: Uuid,
    parent: Uuid,
    kind: &'static str,
}

/// Upsert every seeded `parent_link` row.
///
/// # Errors
/// Propagates any Postgres error from the `INSERT … ON CONFLICT … DO
/// UPDATE` statements.
#[allow(
    clippy::too_many_lines,
    reason = "static table of 22 parent-link rows; splitting hurts readability"
)]
pub async fn seed_parent_links(pool: &PgPool) -> anyhow::Result<()> {
    let rows: [ParentLinkSeed; 22] = [
        // Klaus + Anna lineage.
        ParentLinkSeed {
            child: SEED_PERSON_KLAUS_ID,
            parent: SEED_PERSON_OTTO_ID,
            kind: "biological",
        },
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
        ParentLinkSeed {
            child: SEED_PERSON_ANNA_ID,
            parent: SEED_PERSON_GRETA_ID,
            kind: "biological",
        },
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
        ParentLinkSeed {
            child: SEED_PERSON_LINA_ID,
            parent: SEED_PERSON_KLAUS_ID,
            kind: "biological",
        },
        ParentLinkSeed {
            child: SEED_PERSON_LINA_ID,
            parent: SEED_PERSON_ANNA_ID,
            kind: "biological",
        },
        ParentLinkSeed {
            child: SEED_PERSON_MAX_ID,
            parent: SEED_PERSON_KLAUS_ID,
            kind: "biological",
        },
        ParentLinkSeed {
            child: SEED_PERSON_MAX_ID,
            parent: SEED_PERSON_ANNA_ID,
            kind: "biological",
        },
        ParentLinkSeed {
            child: SEED_PERSON_MIA_ID,
            parent: SEED_PERSON_KLAUS_ID,
            kind: "biological",
        },
        ParentLinkSeed {
            child: SEED_PERSON_MIA_ID,
            parent: SEED_PERSON_ANNA_ID,
            kind: "biological",
        },
        // Lena adopted by both Sabine and Julia.
        ParentLinkSeed {
            child: SEED_PERSON_LENA_ID,
            parent: SEED_PERSON_SABINE_ID,
            kind: "adoptive",
        },
        ParentLinkSeed {
            child: SEED_PERSON_LENA_ID,
            parent: SEED_PERSON_JULIA_ID,
            kind: "adoptive",
        },
        // Tom — single-parent (Markus only).
        ParentLinkSeed {
            child: SEED_PERSON_TOM_ID,
            parent: SEED_PERSON_MARKUS_ID,
            kind: "biological",
        },
        // G4 grandchildren — Lina is sole listed parent (single mother).
        ParentLinkSeed {
            child: SEED_PERSON_EMMA_ID,
            parent: SEED_PERSON_LINA_ID,
            kind: "biological",
        },
        ParentLinkSeed {
            child: SEED_PERSON_NOAH_ID,
            parent: SEED_PERSON_LINA_ID,
            kind: "biological",
        },
    ];
    for r in rows {
        sqlx::query(
            "INSERT INTO parent_links (child_id, parent_id, kind, note) \
             VALUES ($1, $2, ($3::text)::parent_link_kind, '') \
             ON CONFLICT (child_id, parent_id) DO UPDATE SET \
                 kind = EXCLUDED.kind, \
                 note = EXCLUDED.note",
        )
        .bind(r.child)
        .bind(r.parent)
        .bind(r.kind)
        .execute(pool)
        .await?;
    }
    Ok(())
}

/// One partnership row. `started_on` + `ended_on` + `end_reason` cover
/// the open / divorced / widowed cases the FE relations panel renders.
struct PartnershipSeed {
    id: Uuid,
    partner_a: Uuid,
    partner_b: Uuid,
    kind: &'static str,
    started_on: Option<NaiveDate>,
    ended_on: Option<NaiveDate>,
    end_reason: Option<&'static str>,
    note: &'static str,
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

/// Upsert every seeded partnership row.
///
/// # Errors
/// Propagates any Postgres error from the `INSERT … ON CONFLICT (id) DO
/// UPDATE` statements.
#[allow(
    clippy::too_many_lines,
    reason = "static table of 6 partnership rows + per-pair pre-ordering; splitting hurts readability"
)]
pub async fn seed_partnerships(pool: &PgPool) -> anyhow::Result<()> {
    // Wipe any partnerships in the seeded family that ARE NOT one of the
    // hardcoded seed rows. Reason: the historical seed inserted Otto +
    // Hannelore etc. with `gen_random_uuid()` ids and relied on a partial
    // unique index for upsert. Switching to hardcoded `id` upserts means
    // those legacy random-id rows would now conflict on the partial index
    // `partnerships_unique_open (a, b, kind) WHERE ended_on IS NULL`. The
    // DELETE also nukes any user-added partnerships on a `seeder` re-run
    // — that's deliberate, the seeder is a reset.
    let seed_ids: [Uuid; 11] = [
        SEED_PARTNERSHIP_OTTO_HANNELORE_ID,
        SEED_PARTNERSHIP_WERNER_GRETA_ID,
        SEED_PARTNERSHIP_KLAUS_ANNA_ID,
        SEED_PARTNERSHIP_FRIEDRICH_LOTTE_ID,
        SEED_PARTNERSHIP_KLAUS_BRIGITTE_ID,
        SEED_PARTNERSHIP_SABINE_JULIA_ID,
        SEED_PARTNERSHIP_KLAUS_KARIN_ID,
        SEED_PARTNERSHIP_KLAUS_YUKI_ID,
        SEED_PARTNERSHIP_SVEN_MAREN_ID,
        SEED_PARTNERSHIP_HEINZ_URSULA_ID,
        SEED_PARTNERSHIP_LARS_METTE_ID,
    ];
    sqlx::query("DELETE FROM partnerships WHERE family_id = $1 AND id <> ALL($2)")
        .bind(SEED_FAMILY_ID)
        .bind(&seed_ids[..])
        .execute(pool)
        .await?;

    // Pre-order each partner pair so the `partner_a_id < partner_b_id`
    // CHECK constraint is satisfied byte-for-byte in seeded data.
    let (otto, hannelore) = order_pair(SEED_PERSON_OTTO_ID, SEED_PERSON_HANNELORE_ID);
    let (werner, greta) = order_pair(SEED_PERSON_WERNER_ID, SEED_PERSON_GRETA_ID);
    let (klaus, anna) = order_pair(SEED_PERSON_KLAUS_ID, SEED_PERSON_ANNA_ID);
    let (friedrich, lotte) = order_pair(SEED_PERSON_FRIEDRICH_ID, SEED_PERSON_LOTTE_ID);
    let (klaus_b, brigitte) = order_pair(SEED_PERSON_KLAUS_ID, SEED_PERSON_BRIGITTE_ID);
    let (sabine, julia) = order_pair(SEED_PERSON_SABINE_ID, SEED_PERSON_JULIA_ID);
    let (klaus_k, karin) = order_pair(SEED_PERSON_KLAUS_ID, SEED_PERSON_KARIN_ID);
    let (klaus_y, yuki) = order_pair(SEED_PERSON_KLAUS_ID, SEED_PERSON_YUKI_ID);
    let (sven, maren) = order_pair(SEED_PERSON_SVEN_ID, SEED_PERSON_MAREN_ID);
    let (heinz, ursula) = order_pair(SEED_PERSON_HEINZ_ID, SEED_PERSON_URSULA_ID);
    let (lars, mette) = order_pair(SEED_PERSON_LARS_ID, SEED_PERSON_METTE_ID);

    let rows: [PartnershipSeed; 11] = [
        PartnershipSeed {
            id: SEED_PARTNERSHIP_OTTO_HANNELORE_ID,
            partner_a: otto,
            partner_b: hannelore,
            kind: "marriage",
            started_on: Some(ymd(1962, 6, 9)),
            ended_on: None,
            end_reason: None,
            note: "Married in Hamburg.",
        },
        PartnershipSeed {
            id: SEED_PARTNERSHIP_WERNER_GRETA_ID,
            partner_a: werner,
            partner_b: greta,
            kind: "marriage",
            started_on: Some(ymd(1964, 8, 22)),
            ended_on: None,
            end_reason: None,
            note: "Married in Augsburg.",
        },
        PartnershipSeed {
            id: SEED_PARTNERSHIP_KLAUS_ANNA_ID,
            partner_a: klaus,
            partner_b: anna,
            kind: "civil_union",
            started_on: Some(ymd(2002, 5, 18)),
            ended_on: None,
            end_reason: None,
            note: "Klaus's second partnership; civil union after the 2000 divorce.",
        },
        // Widowed — Lotte died 2018, partnership closed by death.
        PartnershipSeed {
            id: SEED_PARTNERSHIP_FRIEDRICH_LOTTE_ID,
            partner_a: friedrich,
            partner_b: lotte,
            kind: "marriage",
            started_on: Some(ymd(1960, 9, 3)),
            ended_on: Some(ymd(2018, 8, 15)),
            end_reason: Some("death"),
            note: "Married in Stuttgart; ended by Lotte's passing.",
        },
        // Divorced — Klaus + Brigitte, ended 2000.
        PartnershipSeed {
            id: SEED_PARTNERSHIP_KLAUS_BRIGITTE_ID,
            partner_a: klaus_b,
            partner_b: brigitte,
            kind: "marriage",
            started_on: Some(ymd(1990, 4, 14)),
            ended_on: Some(ymd(2000, 6, 30)),
            end_reason: Some("divorce"),
            note: "Klaus's first marriage; ended in divorce after 10 years.",
        },
        // Same-sex civil union, ongoing.
        PartnershipSeed {
            id: SEED_PARTNERSHIP_SABINE_JULIA_ID,
            partner_a: sabine,
            partner_b: julia,
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
            partner_a: klaus_k,
            partner_b: karin,
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
            partner_a: klaus_y,
            partner_b: yuki,
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
            partner_a: sven,
            partner_b: maren,
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
            partner_a: heinz,
            partner_b: ursula,
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
            partner_a: lars,
            partner_b: mette,
            kind: "partnership",
            started_on: Some(ymd(2010, 4, 2)),
            ended_on: Some(ymd(2018, 9, 30)),
            end_reason: Some("separation"),
            note: "Separated non-marriage partnership demo (Lars + Mette).",
        },
    ];
    for p in rows {
        sqlx::query(
            "INSERT INTO partnerships \
                 (id, family_id, partner_a_id, partner_b_id, kind, started_on, ended_on, \
                  end_reason, note) \
             VALUES ($1, $2, $3, $4, ($5::text)::partnership_kind, $6, $7, \
                     $8::text::partnership_end_reason, $9) \
             ON CONFLICT (id) DO UPDATE SET \
                 family_id = EXCLUDED.family_id, \
                 partner_a_id = EXCLUDED.partner_a_id, \
                 partner_b_id = EXCLUDED.partner_b_id, \
                 kind = EXCLUDED.kind, \
                 started_on = EXCLUDED.started_on, \
                 ended_on = EXCLUDED.ended_on, \
                 end_reason = EXCLUDED.end_reason, \
                 note = EXCLUDED.note",
        )
        .bind(p.id)
        .bind(SEED_FAMILY_ID)
        .bind(p.partner_a)
        .bind(p.partner_b)
        .bind(p.kind)
        .bind(p.started_on)
        .bind(p.ended_on)
        .bind(p.end_reason)
        .bind(p.note)
        .execute(pool)
        .await?;
    }
    Ok(())
}

/// Return `(min, max)` of the two UUIDs so partnership rows satisfy the
/// `partner_a_id < partner_b_id` CHECK constraint.
const fn order_pair(a: Uuid, b: Uuid) -> (Uuid, Uuid) {
    if a.as_u128() < b.as_u128() { (a, b) } else { (b, a) }
}

#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use super::order_pair;

    #[test]
    fn order_pair_returns_min_max() {
        let a = Uuid::from_u128(1);
        let b = Uuid::from_u128(2);
        assert_eq!(order_pair(a, b), (a, b));
        assert_eq!(order_pair(b, a), (a, b));
    }
}
