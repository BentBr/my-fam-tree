//! The seeded Müller family doubles as the kinship demo: this test pins one
//! example per relation type so the compare mode can be tried on `rdt seed`
//! data, and so a seed edit that drops an example fails here. Pure — runs
//! `compute_kinship` over the seed rows, no database.

use my_fam_tree_domain::{
    FamilyId, InLawSide, Kinship, ParentKind, ParentLink, Partnership, PartnershipEndReason,
    PartnershipKind, PersonId, compute_kinship,
};
use uuid::Uuid;

use crate::ids::{
    SEED_FAMILY_ID, SEED_PERSON_ANNA_ID as ANNA, SEED_PERSON_BRIGITTE_ID as BRIGITTE,
    SEED_PERSON_EMMA_ID as EMMA, SEED_PERSON_FELIX_ID as FELIX,
    SEED_PERSON_FRIEDRICH_ID as FRIEDRICH, SEED_PERSON_JULIA_ID as JULIA,
    SEED_PERSON_KARIN_ID as KARIN, SEED_PERSON_KENJI_ID as KENJI, SEED_PERSON_KLAUS_ID as KLAUS,
    SEED_PERSON_LENA_ID as LENA, SEED_PERSON_LINA_ID as LINA, SEED_PERSON_LOTTE_ID as LOTTE,
    SEED_PERSON_MAX_ID as MAX, SEED_PERSON_OTTO_ID as OTTO, SEED_PERSON_PAUL_ID as PAUL,
    SEED_PERSON_SABINE_ID as SABINE, SEED_PERSON_SVEN_ID as SVEN, SEED_PERSON_WERNER_ID as WERNER,
    SEED_PERSON_YUKI_ID as YUKI,
};
use crate::relationships::{parent_link_rows, partnership_rows};

fn links() -> Vec<ParentLink> {
    parent_link_rows()
        .iter()
        .map(|l| ParentLink {
            child_id: PersonId::from_uuid(l.child),
            parent_id: PersonId::from_uuid(l.parent),
            kind: match l.kind {
                "biological" => ParentKind::Biological,
                "legal" => ParentKind::Legal,
                "adoptive" => ParentKind::Adoptive,
                "step" => ParentKind::Step,
                "social" => ParentKind::Social,
                other => panic!("unknown parent-link kind in seed: {other}"),
            },
            note: String::new(),
        })
        .collect()
}

fn partnerships() -> Vec<Partnership> {
    partnership_rows()
        .iter()
        .map(|p| Partnership {
            id: p.id,
            family_id: FamilyId::from_uuid(SEED_FAMILY_ID),
            partner_a_id: PersonId::from_uuid(p.partner_a),
            partner_b_id: PersonId::from_uuid(p.partner_b),
            kind: match p.kind {
                "marriage" => PartnershipKind::Marriage,
                "civil_union" => PartnershipKind::CivilUnion,
                "partnership" => PartnershipKind::Partnership,
                other => panic!("unknown partnership kind in seed: {other}"),
            },
            started_on: p.started_on,
            ended_on: p.ended_on,
            end_reason: p.end_reason.map(|r| match r {
                "divorce" => PartnershipEndReason::Divorce,
                "separation" => PartnershipEndReason::Separation,
                "death" => PartnershipEndReason::Death,
                other => panic!("unknown end reason in seed: {other}"),
            }),
            note: String::new(),
        })
        .collect()
}

/// Every relation between two seeded persons, read from `a` and naming `b`.
fn relations(a: Uuid, b: Uuid) -> Vec<Kinship> {
    compute_kinship(&links(), &partnerships(), PersonId::from_uuid(a), PersonId::from_uuid(b))
}

/// `(up, down, half, step, in_law)` of each relation that is not a direct partnership.
type Shape = (u8, u8, bool, bool, Option<InLawSide>);

fn shapes(a: Uuid, b: Uuid) -> Vec<Shape> {
    relations(a, b).iter().map(|k| (k.up, k.down, k.half, k.step, k.in_law)).collect()
}

const fn blood(up: u8, down: u8) -> Shape {
    (up, down, false, false, None)
}

const OWN: Option<InLawSide> = Some(InLawSide::ViaOwnPartner);
const RELATIVES: Option<InLawSide> = Some(InLawSide::ViaRelativesPartner);

#[test]
fn seed_family_has_one_example_per_blood_relation() {
    let cases = [
        ("Klaus → Otto: father", KLAUS, OTTO, blood(1, 0)),
        ("Lina → Otto: grandfather", LINA, OTTO, blood(2, 0)),
        ("Emma → Otto: great-grandfather", EMMA, OTTO, blood(3, 0)),
        ("Otto → Emma: great-granddaughter", OTTO, EMMA, blood(0, 3)),
        ("Lina → Max: brother", LINA, MAX, blood(1, 1)),
        ("Lina → Felix: half-brother (shared father)", LINA, FELIX, (1, 1, true, false, None)),
        ("Emma → Max: uncle", EMMA, MAX, blood(2, 1)),
        ("Emma → Sabine: great-aunt", EMMA, SABINE, blood(3, 1)),
        ("Max → Emma: niece", MAX, EMMA, blood(1, 2)),
        ("Sabine → Emma: great-niece", SABINE, EMMA, blood(1, 3)),
        ("Lina → Lena: first cousin (adoptive link counts)", LINA, LENA, blood(2, 2)),
        ("Lena → Emma: first cousin once removed", LENA, EMMA, blood(2, 3)),
        ("Emma → Paul: second cousin", EMMA, PAUL, blood(3, 3)),
    ];
    for (what, a, b, expected) in cases {
        assert_eq!(shapes(a, b), vec![expected], "{what}");
    }
}

#[test]
fn seed_family_has_one_example_per_partnership_state() {
    let state = |a: Uuid, b: Uuid| {
        let rels = relations(a, b);
        assert_eq!(rels.len(), 1, "exactly one relation expected, got {rels:?}");
        rels[0].partnership.map(|p| (p.is_open(), p.end_reason))
    };
    assert_eq!(state(KLAUS, ANNA), Some((true, None)), "current partner");
    assert_eq!(state(KLAUS, YUKI), Some((true, None)), "second current partner");
    assert_eq!(
        state(KLAUS, BRIGITTE),
        Some((false, Some(PartnershipEndReason::Divorce))),
        "divorced"
    );
    assert_eq!(
        state(KLAUS, KARIN),
        Some((false, Some(PartnershipEndReason::Separation))),
        "separated"
    );
    assert_eq!(
        state(FRIEDRICH, LOTTE),
        Some((false, Some(PartnershipEndReason::Death))),
        "widowed"
    );
}

#[test]
fn seed_family_has_one_example_per_in_law_relation() {
    let cases = [
        ("Klaus → Werner: father-in-law", KLAUS, WERNER, (1, 0, false, false, OWN)),
        ("Werner → Klaus: son-in-law", WERNER, KLAUS, (0, 1, false, false, RELATIVES)),
        ("Klaus → Sabine: partner's sister", KLAUS, SABINE, (1, 1, false, false, OWN)),
        ("Anna → Julia: sister's partner", ANNA, JULIA, (1, 1, false, false, RELATIVES)),
        ("Lina → Julia: aunt by marriage", LINA, JULIA, (2, 1, false, false, RELATIVES)),
        ("Klaus → Lena: partner's niece", KLAUS, LENA, (1, 2, false, false, OWN)),
    ];
    for (what, a, b, expected) in cases {
        assert_eq!(shapes(a, b), vec![expected], "{what}");
    }
}

#[test]
fn seed_family_has_one_example_per_step_relation() {
    let cases = [
        ("Felix → Anna: step-mother (explicit step link)", FELIX, ANNA, (1, 0, false, true, None)),
        ("Felix → Werner: step-grandfather", FELIX, WERNER, (2, 0, false, true, None)),
        ("Lina → Yuki: step-mother (father's partner)", LINA, YUKI, (1, 0, false, true, None)),
        ("Yuki → Lina: step-daughter", YUKI, LINA, (0, 1, false, true, None)),
        ("Lina → Kenji: step-brother", LINA, KENJI, (1, 1, false, true, None)),
    ];
    for (what, a, b, expected) in cases {
        assert_eq!(shapes(a, b), vec![expected], "{what}");
    }
}

#[test]
fn seed_family_has_examples_of_no_relation() {
    assert_eq!(shapes(LINA, BRIGITTE), vec![], "father's divorced ex-wife is no relation");
    assert_eq!(shapes(ANNA, YUKI), vec![], "two partners of the same person are no relation");
    assert_eq!(shapes(SVEN, OTTO), vec![], "a standalone couple is unrelated to the Müllers");
}
