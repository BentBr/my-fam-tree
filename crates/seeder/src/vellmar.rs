//! Second seeded family — "Vellmar".
//!
//! An ANONYMIZED mirror of a real production tree. Names and birthplaces
//! are fully changed; the graph STRUCTURE (parent links, partnership
//! open/ended state) and the birth DATES are preserved so the layout
//! engine hits the exact same code paths the real tree exercises. The
//! cases this family reproduces:
//!
//! - Multi-couple chain: Gesa has TWO concurrent OPEN marriages (Bodo +
//!   Berthold) → chain `[Bodo, Gesa, Berthold]`.
//! - In-married sibling: Jens (older) is married to Karla, whose id
//!   sorts first, so his couple block leads with the married-in spouse.
//!   He must still sort by his own birth date among his siblings.
//! - Bridging couple: Arvid (a Vellmar child) married Marit, the blood
//!   daughter of the separate leaf-root Quanten couple. The couple must
//!   hang under the more-connected Vellmar side, not be banished to the
//!   Quanten cluster.
//! - Two-grandmother roots: Astrid (Gesa's mother) and Wilma
//!   (Berthold's mother) each sit above their own child in the chain.
//!
//! UUIDs are hand-assigned so the intra-couple id order matches prod
//! (married-in spouse = smaller id where prod had it), keeping the
//! fixture a faithful regression guard.

mod persons;

use my_fam_tree_persistence::seed::SeedDb;
use uuid::Uuid;

use crate::ids::SEED_FAMILY_VELLMAR_ID;
use crate::persons::{delete_non_seed_persons, upsert_person, ymd};
use crate::relationships::{
    ParentLinkSeed, PartnershipSeed, delete_non_seed_partnerships, order_pair, upsert_parent_link,
    upsert_partnership,
};

// Persons (id suffix 05xx). Ordering constraints encoded in the suffixes:
//   Bodo(04) < Berthold(05)         → chain reads [Bodo, Gesa, Berthold]
//   Karla(0a) < Jens(0b)            → Jens's couple leads with married-in Karla
//   Marit(11) < Arvid(12)           → bridging couple leads with the Quanten blood child
const P_ASTRID: Uuid = Uuid::from_u128(0x0000_0003_0000_0000_0000_0000_0000_0501); // Gesa's mother
const P_WILMA: Uuid = Uuid::from_u128(0x0000_0003_0000_0000_0000_0000_0000_0502); // Berthold's mother
const P_GESA: Uuid = Uuid::from_u128(0x0000_0003_0000_0000_0000_0000_0000_0503); // anchor
const P_BODO: Uuid = Uuid::from_u128(0x0000_0003_0000_0000_0000_0000_0000_0504); // open partner 1
const P_BERTHOLD: Uuid = Uuid::from_u128(0x0000_0003_0000_0000_0000_0000_0000_0505); // open partner 2
const P_QUIRIN: Uuid = Uuid::from_u128(0x0000_0003_0000_0000_0000_0000_0000_0506); // Quanten anchor
const P_VRENI: Uuid = Uuid::from_u128(0x0000_0003_0000_0000_0000_0000_0000_0507);
const P_SARI: Uuid = Uuid::from_u128(0x0000_0003_0000_0000_0000_0000_0000_0508);
const P_ANNIKA: Uuid = Uuid::from_u128(0x0000_0003_0000_0000_0000_0000_0000_0509);
const P_KARLA: Uuid = Uuid::from_u128(0x0000_0003_0000_0000_0000_0000_0000_050a);
const P_JENS: Uuid = Uuid::from_u128(0x0000_0003_0000_0000_0000_0000_0000_050b);
const P_BJARNE: Uuid = Uuid::from_u128(0x0000_0003_0000_0000_0000_0000_0000_050c);
const P_IDA: Uuid = Uuid::from_u128(0x0000_0003_0000_0000_0000_0000_0000_050d);
// "Alma Schober" — the same person also appears as a separate row in the
// Lang family (mirrors a real prod individual recorded under two
// family_ids). Distinct id per family, same display name.
const P_ALMA: Uuid = Uuid::from_u128(0x0000_0003_0000_0000_0000_0000_0000_050e);
const P_MARA: Uuid = Uuid::from_u128(0x0000_0003_0000_0000_0000_0000_0000_050f);
const P_DORIAN: Uuid = Uuid::from_u128(0x0000_0003_0000_0000_0000_0000_0000_0510);
const P_MARIT: Uuid = Uuid::from_u128(0x0000_0003_0000_0000_0000_0000_0000_0511);
const P_ARVID: Uuid = Uuid::from_u128(0x0000_0003_0000_0000_0000_0000_0000_0512);
const P_BENTE: Uuid = Uuid::from_u128(0x0000_0003_0000_0000_0000_0000_0000_0513);
const P_ALEA: Uuid = Uuid::from_u128(0x0000_0003_0000_0000_0000_0000_0000_0514);
const P_LIA: Uuid = Uuid::from_u128(0x0000_0003_0000_0000_0000_0000_0000_0515);
const P_JUNA: Uuid = Uuid::from_u128(0x0000_0003_0000_0000_0000_0000_0000_0516);
const P_KIRA: Uuid = Uuid::from_u128(0x0000_0003_0000_0000_0000_0000_0000_0517);
const P_MAJA: Uuid = Uuid::from_u128(0x0000_0003_0000_0000_0000_0000_0000_0518);

// Partnerships (id suffix 05xx).
const PA_BODO_GESA: Uuid = Uuid::from_u128(0x0000_0004_0000_0000_0000_0000_0000_0501);
const PA_GESA_BERTHOLD: Uuid = Uuid::from_u128(0x0000_0004_0000_0000_0000_0000_0000_0502);
const PA_QUIRIN_VRENI: Uuid = Uuid::from_u128(0x0000_0004_0000_0000_0000_0000_0000_0503);
const PA_QUIRIN_SARI: Uuid = Uuid::from_u128(0x0000_0004_0000_0000_0000_0000_0000_0504);
const PA_JENS_KARLA: Uuid = Uuid::from_u128(0x0000_0004_0000_0000_0000_0000_0000_0505);
const PA_BJARNE_IDA: Uuid = Uuid::from_u128(0x0000_0004_0000_0000_0000_0000_0000_0506);
const PA_BJARNE_ALMA: Uuid = Uuid::from_u128(0x0000_0004_0000_0000_0000_0000_0000_0507);
const PA_MARA_DORIAN: Uuid = Uuid::from_u128(0x0000_0004_0000_0000_0000_0000_0000_0508);
const PA_ARVID_MARIT: Uuid = Uuid::from_u128(0x0000_0004_0000_0000_0000_0000_0000_0509);

/// Number of persons this family seeds (summed into the seed report).
pub const PERSON_COUNT: usize = 24;
/// Number of parent-link rows this family seeds (row-count assert only).
pub const PARENT_LINK_COUNT: i64 = 25;
/// Number of partnership rows this family seeds (row-count assert only).
pub const PARTNERSHIP_COUNT: i64 = 9;

fn parent_links() -> Vec<ParentLinkSeed> {
    let bio = |child: Uuid, parent: Uuid| ParentLinkSeed { child, parent, kind: "biological" };
    vec![
        bio(P_GESA, P_ASTRID),
        bio(P_BERTHOLD, P_WILMA),
        bio(P_ANNIKA, P_BODO),
        bio(P_ANNIKA, P_GESA),
        bio(P_JENS, P_BODO),
        bio(P_JENS, P_GESA),
        bio(P_BJARNE, P_GESA),
        bio(P_BJARNE, P_BERTHOLD),
        bio(P_MARA, P_GESA),
        bio(P_MARA, P_BERTHOLD),
        bio(P_ARVID, P_GESA),
        bio(P_ARVID, P_BERTHOLD),
        bio(P_BENTE, P_BJARNE),
        bio(P_BENTE, P_IDA),
        bio(P_ALEA, P_ARVID),
        bio(P_ALEA, P_MARIT),
        bio(P_LIA, P_ARVID),
        bio(P_LIA, P_MARIT),
        bio(P_MARIT, P_QUIRIN),
        bio(P_MARIT, P_SARI),
        bio(P_JUNA, P_QUIRIN),
        bio(P_JUNA, P_SARI),
        bio(P_KIRA, P_QUIRIN),
        bio(P_KIRA, P_SARI),
        bio(P_MAJA, P_KIRA),
    ]
}

fn partnerships() -> Vec<PartnershipSeed> {
    let (bodo, gesa1) = order_pair(P_BODO, P_GESA);
    let (gesa2, berthold) = order_pair(P_GESA, P_BERTHOLD);
    let (quirin1, vreni) = order_pair(P_QUIRIN, P_VRENI);
    let (quirin2, sari) = order_pair(P_QUIRIN, P_SARI);
    let (jens, karla) = order_pair(P_JENS, P_KARLA);
    let (bjarne1, ida) = order_pair(P_BJARNE, P_IDA);
    let (bjarne2, alma) = order_pair(P_BJARNE, P_ALMA);
    let (mara, dorian) = order_pair(P_MARA, P_DORIAN);
    let (arvid, marit) = order_pair(P_ARVID, P_MARIT);
    let open =
        |id: Uuid, a: Uuid, b: Uuid, kind: &'static str, note: &'static str| PartnershipSeed {
            id,
            partner_a: a,
            partner_b: b,
            kind,
            started_on: None,
            ended_on: None,
            end_reason: None,
            note,
        };
    vec![
        open(PA_BODO_GESA, bodo, gesa1, "marriage", "Gesa's first open marriage."),
        open(PA_GESA_BERTHOLD, gesa2, berthold, "marriage", "Gesa's second open marriage."),
        open(PA_QUIRIN_VRENI, quirin1, vreni, "marriage", "Quirin + Vreni (open)."),
        open(PA_QUIRIN_SARI, quirin2, sari, "marriage", "Quirin + Sari (open)."),
        open(PA_JENS_KARLA, jens, karla, "marriage", "Jens + Karla (open)."),
        PartnershipSeed {
            id: PA_BJARNE_IDA,
            partner_a: bjarne1,
            partner_b: ida,
            kind: "marriage",
            started_on: Some(ymd(2012, 1, 5)),
            ended_on: Some(ymd(2014, 12, 15)),
            end_reason: Some("divorce"),
            note: "Bjarne + Ida; divorced.",
        },
        PartnershipSeed {
            id: PA_BJARNE_ALMA,
            partner_a: bjarne2,
            partner_b: alma,
            kind: "partnership",
            started_on: Some(ymd(2025, 9, 14)),
            ended_on: None,
            end_reason: None,
            note: "Bjarne's current partner.",
        },
        open(PA_MARA_DORIAN, mara, dorian, "marriage", "Mara + Dorian (open)."),
        PartnershipSeed {
            id: PA_ARVID_MARIT,
            partner_a: arvid,
            partner_b: marit,
            kind: "marriage",
            started_on: Some(ymd(2018, 1, 14)),
            ended_on: None,
            end_reason: None,
            note: "Arvid + Marit; bridges the Vellmar + Quanten families.",
        },
    ]
}

/// Seed the Vellmar family: persons, parent links, partnerships. Each
/// step resets its own family-scoped rows first so a re-seed is a clean
/// reset (matching the Müller seed's behaviour).
///
/// # Errors
/// Propagates any Postgres error from the upserts.
pub async fn seed(db: &SeedDb<'_>) -> anyhow::Result<()> {
    let rows = persons::persons();
    let keep: Vec<Uuid> = rows.iter().map(|p| p.id).collect();
    delete_non_seed_persons(db, SEED_FAMILY_VELLMAR_ID, &keep).await?;
    for p in &rows {
        upsert_person(db, SEED_FAMILY_VELLMAR_ID, p).await?;
    }

    let parts = partnerships();
    let part_keep: Vec<Uuid> = parts.iter().map(|p| p.id).collect();
    delete_non_seed_partnerships(db, SEED_FAMILY_VELLMAR_ID, &part_keep).await?;
    for p in &parts {
        upsert_partnership(db, SEED_FAMILY_VELLMAR_ID, p).await?;
    }

    for r in &parent_links() {
        upsert_parent_link(db, r).await?;
    }
    Ok(())
}
