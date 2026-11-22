//! Kinship computation: the relations between two persons in a family.
//!
//! See `.claude/specs/kinship-comparison.md` for the full semantics. Pure
//! logic over the family's parent links + partnerships — no I/O, unit-tested
//! without a database.

mod affinal;
mod graph;

use std::cmp::Reverse;
use std::collections::HashSet;

use affinal::{HopPartners, derived_step, direct_partners, in_laws};
use graph::{Graph, blood_relations};
use time::Date;
use uuid::Uuid;

use crate::{ParentLink, Partnership, PartnershipEndReason, PartnershipKind, PersonId};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InLawSide {
    /// B is a blood relative of A's partner (mother-in-law, partner's sibling).
    ViaOwnPartner,
    /// B is the partner of A's blood relative (son-in-law, sibling's partner).
    ViaRelativesPartner,
}

impl InLawSide {
    const fn flipped(self) -> Self {
        match self {
            Self::ViaOwnPartner => Self::ViaRelativesPartner,
            Self::ViaRelativesPartner => Self::ViaOwnPartner,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PartnerInfo {
    pub kind: PartnershipKind,
    pub ended_on: Option<Date>,
    pub end_reason: Option<PartnershipEndReason>,
}

impl PartnerInfo {
    /// Open = neither an end date nor an end reason. The API accepts a
    /// reason without a date; such a row counts as ended.
    #[must_use]
    pub const fn is_open(&self) -> bool {
        self.ended_on.is_none() && self.end_reason.is_none()
    }
}

/// One relation, read from A's perspective and naming B.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Kinship {
    /// Generational steps from A up to the common ancestor.
    pub up: u8,
    /// Steps from the ancestor down to B.
    pub down: u8,
    /// `(1,1)` only: one shared parent AND each has another, different parent.
    pub half: bool,
    /// Path crosses a step parent-link or a derived step edge.
    pub step: bool,
    pub in_law: Option<InLawSide>,
    /// Set when A and B are direct partners (`up == down == 0`).
    pub partnership: Option<PartnerInfo>,
    /// The ONE partnership row the path crosses: the partnership itself for
    /// direct partners, the hop for in-law and derived step relations.
    /// `None` for blood relations. Tells the renderer which partner edge to
    /// light when two persons share several partnership rows.
    pub via_partnership: Option<Uuid>,
    /// `[A, …, B]`, no person twice, every consecutive pair a real edge:
    /// parent links, plus the `via_partnership` edge where set.
    pub path: Vec<PersonId>,
}

/// The distinct relations between `a` and `b`; empty when unrelated or
/// `a == b`.
///
/// One relation per lowest common ancestor (ancestor couples collapse), each
/// partnership row between the two, in-laws through exactly one partner hop
/// and shallow derived step relations. Order: blood relations first, then
/// direct partners, then in-law / step relations — by distance WITHIN each
/// group, so a cousin is listed before a spouse.
///
/// Symmetric by construction: the pair is evaluated in id order and mirrored
/// for the other direction, so `compute_kinship(b, a)` is the exact mirror of
/// `compute_kinship(a, b)` — up/down swapped, in-law side flipped, path
/// reversed, same order.
#[must_use]
pub fn compute_kinship(
    parent_links: &[ParentLink],
    partnerships: &[Partnership],
    a: PersonId,
    b: PersonId,
) -> Vec<Kinship> {
    if a == b {
        return Vec::new();
    }
    if a.into_uuid() > b.into_uuid() {
        return compute_ordered(parent_links, partnerships, b, a)
            .into_iter()
            .map(mirrored)
            .collect();
    }
    compute_ordered(parent_links, partnerships, a, b)
}

fn compute_ordered(
    parent_links: &[ParentLink],
    partnerships: &[Partnership],
    a: PersonId,
    b: PersonId,
) -> Vec<Kinship> {
    let g = Graph::new(parent_links);
    let hops = HopPartners::new(partnerships);
    let mut out: Vec<Kinship> = blood_relations(&g, a, b)
        .into_iter()
        .map(|r| Kinship {
            up: r.up,
            down: r.down,
            half: r.half,
            step: r.step,
            in_law: None,
            partnership: None,
            via_partnership: None,
            path: r.path,
        })
        .collect();
    out.extend(direct_partners(partnerships, a, b));
    out.extend(in_laws(&g, &hops, a, b));
    out.extend(derived_step(&g, &hops, a, b));
    // A route that visits a person twice is not an independent relation.
    out.retain(|k| {
        let mut seen = HashSet::new();
        k.path.iter().all(|p| seen.insert(*p))
    });
    sort_and_dedup(&mut out);
    out
}

fn mirrored(mut k: Kinship) -> Kinship {
    std::mem::swap(&mut k.up, &mut k.down);
    k.in_law = k.in_law.map(InLawSide::flipped);
    k.path.reverse();
    k
}

/// Sort group: pure blood, then direct partner, then in-law / step.
const fn rank(k: &Kinship) -> u8 {
    if k.partnership.is_some() {
        1
    } else if k.in_law.is_some() || k.step {
        2
    } else {
        0
    }
}

/// Total order over the DESCRIPTOR (everything but the path): group, then
/// distance, then tie-breakers. Equal keys <=> equal descriptors, so the
/// same key drives both the sort and the dedup. Partnerships between the
/// same pair list the open one first, then the most recently ended.
fn descriptor_key(k: &Kinship) -> impl Ord + use<> {
    let in_law = match k.in_law {
        None => 0_u8,
        Some(InLawSide::ViaOwnPartner) => 1,
        Some(InLawSide::ViaRelativesPartner) => 2,
    };
    let partner = k.partnership.map(|p| {
        (
            !p.is_open(),
            Reverse(p.ended_on),
            p.kind.as_db(),
            p.end_reason.map(PartnershipEndReason::as_db),
        )
    });
    (
        rank(k),
        u16::from(k.up).saturating_add(u16::from(k.down)),
        k.up,
        k.half,
        k.step,
        in_law,
        partner,
    )
}

/// Order by descriptor, shortest path first within one descriptor (ties by
/// path ids, so the result never depends on hash-map iteration order), then
/// keep the first — i.e. shortest — path per descriptor. The partnership id
/// is the last tie-breaker so the kept route is stable too.
fn sort_and_dedup(out: &mut Vec<Kinship>) {
    out.sort_by(|x, y| {
        descriptor_key(x)
            .cmp(&descriptor_key(y))
            .then_with(|| x.path.len().cmp(&y.path.len()))
            .then_with(|| {
                x.path.iter().map(PersonId::as_uuid).cmp(y.path.iter().map(PersonId::as_uuid))
            })
            .then_with(|| x.via_partnership.cmp(&y.via_partnership))
    });
    out.dedup_by(|later, kept| descriptor_key(later) == descriptor_key(kept));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dates::date_from_ymd;
    use crate::{FamilyId, ParentKind};

    fn pid(n: u8) -> PersonId {
        let mut bytes = [0u8; 16];
        bytes[15] = n;
        PersonId::from_uuid(Uuid::from_bytes(bytes))
    }

    fn link(child: u8, parent: u8) -> ParentLink {
        ParentLink {
            child_id: pid(child),
            parent_id: pid(parent),
            kind: ParentKind::Biological,
            note: String::new(),
        }
    }

    fn step_link(child: u8, parent: u8) -> ParentLink {
        ParentLink { kind: ParentKind::Step, ..link(child, parent) }
    }

    fn married(a: u8, b: u8) -> Partnership {
        Partnership {
            id: Uuid::from_bytes([a; 16]),
            family_id: FamilyId::from_uuid(Uuid::from_bytes([9u8; 16])),
            partner_a_id: pid(a),
            partner_b_id: pid(b),
            kind: PartnershipKind::Marriage,
            started_on: None,
            ended_on: None,
            end_reason: None,
            note: String::new(),
        }
    }

    /// Id of the partnership row `married(a, _)` / `ended(_, b, ..)` builds.
    fn row(n: u8) -> Uuid {
        Uuid::from_bytes([n; 16])
    }

    /// Ended partnership; its row id is derived from `b` so it differs from `married(a, b)`.
    fn ended(a: u8, b: u8, year: i32, reason: PartnershipEndReason) -> Partnership {
        Partnership {
            id: Uuid::from_bytes([b; 16]),
            ended_on: date_from_ymd(year, 1, 1),
            end_reason: Some(reason),
            ..married(a, b)
        }
    }

    /// Fixture family (numbers are `pid(n)`):
    ///   1=GreatGrandpa
    ///   2=Grandpa(child of 1), 3=Grandma            — couple
    ///   4=Dad(child of 2+3), 5=UncleBob(child of 2+3), 13=GreatAunt(child of 1)
    ///   6=Mom, 7=Me(child of 4+6), 8=Sis(child of 4+6), 9=HalfBro(child of 4+16)
    ///   16=HalfBro's mother, 17=Sib(child of 4 only), 18=Sib(child of 4 only)
    ///   10=Cousin(child of 5), 14=SecondCousin(grandchild of 13 via 15)
    ///   20=Spouse(child of 21), 22=SpouseSibling(child of 21), 23=SisHusband
    ///   30=DadsNewWife, 31=her child
    fn links() -> Vec<ParentLink> {
        vec![
            link(2, 1),
            link(13, 1),
            link(4, 2),
            link(4, 3),
            link(5, 2),
            link(5, 3),
            link(7, 4),
            link(7, 6),
            link(8, 4),
            link(8, 6),
            link(9, 4),
            link(9, 16),
            link(17, 4),
            link(18, 4),
            link(10, 5),
            link(15, 13),
            link(14, 15),
            link(20, 21),
            link(22, 21),
            link(31, 30),
        ]
    }

    /// Me+Spouse, Sis+SisHusband, Dad+NewWife (Dad and Mom are not partnered).
    fn parts() -> Vec<Partnership> {
        vec![married(7, 20), married(8, 23), married(4, 30)]
    }

    type Shape = (u8, u8, bool, bool, Option<InLawSide>);

    fn shape(k: &Kinship) -> Shape {
        (k.up, k.down, k.half, k.step, k.in_law)
    }

    const fn plain(up: u8, down: u8) -> Shape {
        (up, down, false, false, None)
    }

    fn only(rels: Vec<Kinship>) -> Kinship {
        assert_eq!(rels.len(), 1, "expected exactly one relation, got {rels:?}");
        rels.into_iter().next().unwrap()
    }

    fn assert_unrelated(rels: &[Kinship]) {
        assert_eq!(rels, Vec::<Kinship>::new(), "expected no relation");
    }

    /// Parent links only — no partnerships.
    fn blood(a: u8, b: u8) -> Kinship {
        only(compute_kinship(&links(), &[], pid(a), pid(b)))
    }

    fn full(a: u8, b: u8) -> Vec<Kinship> {
        compute_kinship(&links(), &parts(), pid(a), pid(b))
    }

    fn path(ids: &[u8]) -> Vec<PersonId> {
        ids.iter().map(|n| pid(*n)).collect()
    }

    #[test]
    fn direct_line_up_and_down() {
        let parent = blood(7, 4);
        assert_eq!(shape(&parent), plain(1, 0));
        assert_eq!(parent.path, path(&[7, 4]));
        let great = blood(7, 1);
        assert_eq!(shape(&great), plain(3, 0));
        assert_eq!(great.path, path(&[7, 4, 2, 1]));
        assert_eq!(shape(&blood(1, 7)), plain(0, 3));
        assert_eq!(great.via_partnership, None);
    }

    #[test]
    fn full_sibling_collapses_ancestor_couple() {
        // Me & Sis share Dad AND Mom -> exactly ONE (1,1), not half, and no
        // spurious cousin relation through the shared grandparents.
        assert_eq!(shape(&blood(7, 8)), plain(1, 1));
    }

    #[test]
    fn half_sibling_is_flagged_when_both_have_a_different_second_parent() {
        // Me (Dad + Mom) and HalfBro (Dad + another mother).
        assert_eq!(shape(&blood(7, 9)), (1, 1, true, false, None));
        assert_eq!(shape(&blood(9, 7)), (1, 1, true, false, None));
    }

    #[test]
    fn sibling_with_a_missing_parent_is_a_plain_sibling() {
        // Only one of the two has a second parent recorded.
        assert_eq!(shape(&blood(7, 17)), plain(1, 1));
        assert_eq!(shape(&blood(17, 7)), plain(1, 1));
        // Both have just the one shared parent recorded.
        assert_eq!(shape(&blood(17, 18)), plain(1, 1));
    }

    #[test]
    fn collateral_lines() {
        assert_eq!(shape(&blood(7, 5)), plain(2, 1)); // uncle
        assert_eq!(shape(&blood(5, 7)), plain(1, 2)); // nephew
        assert_eq!(shape(&blood(7, 13)), plain(3, 1)); // great-aunt
        assert_eq!(shape(&blood(7, 10)), plain(2, 2)); // first cousin
        assert_eq!(shape(&blood(7, 14)), plain(3, 3)); // second cousin
        assert_eq!(shape(&blood(4, 14)), plain(2, 3)); // first cousin once removed
    }

    #[test]
    fn couple_path_is_deterministic() {
        // Grandpa(2) and Grandma(3) are both lowest common ancestors of Me
        // and UncleBob; the path always runs through the lower id.
        for _ in 0..20 {
            assert_eq!(blood(7, 5).path, path(&[7, 4, 2, 5]));
            assert_eq!(blood(5, 7).path, path(&[5, 2, 4, 7]));
        }
    }

    /// Two branches of length 5 under apex 50:
    ///   50 <- 51 <- 52 <- 53 <- 54 <- 60
    ///   50 <- 71 <- 72 <- 73 <- 74 <- 70
    #[test]
    fn distant_cousins() {
        let deep: Vec<ParentLink> = [
            (51, 50),
            (52, 51),
            (53, 52),
            (54, 53),
            (60, 54),
            (71, 50),
            (72, 71),
            (73, 72),
            (74, 73),
            (70, 74),
        ]
        .into_iter()
        .map(|(c, p)| link(c, p))
        .collect();
        let equal = only(compute_kinship(&deep, &[], pid(60), pid(70)));
        assert_eq!(shape(&equal), plain(5, 5));
        assert_eq!(equal.path.len(), 11);
        let unequal = only(compute_kinship(&deep, &[], pid(60), pid(73)));
        assert_eq!(shape(&unequal), plain(5, 3));
    }

    #[test]
    fn explicit_step_link_marks_the_path() {
        let mut with_step = links();
        with_step.push(step_link(7, 11));
        let rel = only(compute_kinship(&with_step, &[], pid(7), pid(11)));
        assert_eq!(shape(&rel), (1, 0, false, true, None));
    }

    #[test]
    fn unrelated_and_same_person_are_empty() {
        assert_unrelated(&compute_kinship(&links(), &parts(), pid(7), pid(99)));
        assert_unrelated(&compute_kinship(&links(), &parts(), pid(7), pid(7)));
        // Co-parents without a partnership row are not related.
        assert_unrelated(&full(4, 6));
    }

    #[test]
    fn direct_partner_active_divorced_widowed() {
        let active = only(full(7, 20));
        assert_eq!((active.up, active.down), (0, 0));
        assert_eq!(active.partnership.map(|p| p.end_reason), Some(None));
        assert_eq!(active.path, path(&[7, 20]));
        assert_eq!(active.via_partnership, Some(row(7)));

        for reason in [PartnershipEndReason::Divorce, PartnershipEndReason::Death] {
            let rel = only(compute_kinship(&[], &[ended(7, 20, 2020, reason)], pid(20), pid(7)));
            assert_eq!(rel.partnership.and_then(|p| p.end_reason), Some(reason));
            assert_eq!(rel.path, path(&[20, 7]));
        }
    }

    #[test]
    fn remarriage_lists_open_partnership_first() {
        let both = vec![ended(7, 20, 2010, PartnershipEndReason::Divorce), married(7, 20)];
        let rels = compute_kinship(&[], &both, pid(7), pid(20));
        assert_eq!(rels.len(), 2, "{rels:?}");
        assert_eq!(rels[0].partnership.map(|p| p.ended_on), Some(None));
        // Same two persons, two rows: each relation names its own row.
        assert_eq!(
            (rels[0].via_partnership, rels[1].via_partnership),
            (Some(row(7)), Some(row(20)))
        );
        assert_eq!(
            rels[1].partnership.and_then(|p| p.end_reason),
            Some(PartnershipEndReason::Divorce)
        );
    }

    #[test]
    fn parent_in_law_and_child_in_law() {
        let mother_in_law = only(full(7, 21));
        assert_eq!(shape(&mother_in_law), (1, 0, false, false, Some(InLawSide::ViaOwnPartner)));
        assert_eq!(mother_in_law.path, path(&[7, 20, 21]));
        assert_eq!(mother_in_law.via_partnership, Some(row(7)));
        let son_in_law = only(full(21, 7));
        assert_eq!(shape(&son_in_law), (0, 1, false, false, Some(InLawSide::ViaRelativesPartner)));
        assert_eq!(son_in_law.path, path(&[21, 20, 7]));
    }

    #[test]
    fn sibling_in_law_both_sides() {
        let partners_sibling = only(full(7, 22));
        assert_eq!(shape(&partners_sibling), (1, 1, false, false, Some(InLawSide::ViaOwnPartner)));
        let siblings_partner = only(full(7, 23));
        assert_eq!(
            shape(&siblings_partner),
            (1, 1, false, false, Some(InLawSide::ViaRelativesPartner))
        );
        assert_eq!(siblings_partner.path, path(&[7, 4, 8, 23]));
    }

    #[test]
    fn uncle_by_marriage() {
        let mut with_aunt = parts();
        with_aunt.push(married(5, 40));
        let rel = only(compute_kinship(&links(), &with_aunt, pid(7), pid(40)));
        assert_eq!(shape(&rel), (2, 1, false, false, Some(InLawSide::ViaRelativesPartner)));
    }

    #[test]
    fn divorce_hop_excluded_death_hop_included() {
        let divorced = [ended(7, 20, 2020, PartnershipEndReason::Divorce)];
        assert_unrelated(&compute_kinship(&links(), &divorced, pid(7), pid(21)));
        let separated = [ended(7, 20, 2020, PartnershipEndReason::Separation)];
        assert_unrelated(&compute_kinship(&links(), &separated, pid(7), pid(21)));
        let widowed = [ended(7, 20, 2020, PartnershipEndReason::Death)];
        let rel = only(compute_kinship(&links(), &widowed, pid(7), pid(21)));
        assert_eq!(rel.in_law, Some(InLawSide::ViaOwnPartner));
    }

    #[test]
    fn co_parent_is_not_child_in_law() {
        // Dad married to Mom, sharing child Me: Dad -> Me stays a plain child.
        let rels = compute_kinship(&links(), &[married(4, 6)], pid(4), pid(7));
        assert_eq!(shape(&only(rels)), plain(0, 1));
    }

    #[test]
    fn derived_step_parent_is_not_a_parent_in_law() {
        // Me vs Dad's new wife: exactly one relation, step — never in-law.
        let step_mother = only(full(7, 30));
        assert_eq!(shape(&step_mother), (1, 0, false, true, None));
        assert_eq!(step_mother.path, path(&[7, 4, 30]));
        assert_eq!(step_mother.via_partnership, Some(row(4)));
        let step_child = only(full(30, 7));
        assert_eq!(shape(&step_child), (0, 1, false, true, None));
        assert_eq!(step_child.path, path(&[30, 4, 7]));
    }

    #[test]
    fn derived_step_child_in_id_order() {
        // Step-parent id (35) below the child id (40): exercises the
        // un-mirrored (0,1) branch.
        let rel = only(compute_kinship(&[link(40, 41)], &[married(35, 41)], pid(35), pid(40)));
        assert_eq!(shape(&rel), (0, 1, false, true, None));
        assert_eq!(rel.path, path(&[35, 41, 40]));
    }

    #[test]
    fn derived_step_sibling() {
        let rel = only(full(7, 31));
        assert_eq!(shape(&rel), (1, 1, false, true, None));
        assert_eq!(rel.path, path(&[7, 4, 30, 31]));
        // Shallow on purpose: the step-mother's parent is nobody to Me.
        let mut deeper = links();
        deeper.push(link(30, 32));
        assert_unrelated(&compute_kinship(&deeper, &parts(), pid(7), pid(32)));
    }

    #[test]
    fn multiple_relations_sorted_blood_partner_then_affinal() {
        // Spouse(20) is ALSO my first cousin (child of UncleBob): the blood
        // relation sorts before the partnership.
        let mut cousin_marriage = links();
        cousin_marriage.push(link(20, 5));
        let rels = compute_kinship(&cousin_marriage, &parts(), pid(7), pid(20));
        assert_eq!(rels.len(), 2, "{rels:?}");
        assert_eq!(shape(&rels[0]), plain(2, 2));
        assert!(rels[0].partnership.is_none());
        assert!(rels[1].partnership.is_some());
    }

    #[test]
    fn ancestor_walk_stops_after_32_generations() {
        // 100 <- 101 <- … <- 133: one unbroken line of 33 parent links.
        let line: Vec<ParentLink> = (100..133).map(|n| link(n, n + 1)).collect();
        let at_cap = only(compute_kinship(&line, &[], pid(100), pid(132)));
        assert_eq!(shape(&at_cap), plain(32, 0));
        assert_unrelated(&compute_kinship(&line, &[], pid(100), pid(133)));
    }

    #[test]
    fn equal_length_blood_route_wins_over_step_route_in_any_row_order() {
        // 1 has a biological parent 2 and a step parent 3; 2 and 3 are both
        // children of 4, and so is 5. Two walks 1 -> 4 of the same length,
        // one of them through the step link: 5 is a plain uncle, whichever
        // order the database returns the rows in.
        let mut rows = vec![link(1, 2), step_link(1, 3), link(2, 4), link(3, 4), link(5, 4)];
        for _ in 0..2 {
            let rel = only(compute_kinship(&rows, &[], pid(1), pid(5)));
            assert_eq!(shape(&rel), plain(2, 1));
            assert_eq!(rel.path, path(&[1, 2, 4, 5]));
            rows.reverse();
        }
    }

    #[test]
    fn parent_who_is_also_the_other_parents_sibling_is_listed_as_parent_only() {
        // 2 and 3 are siblings (children of 4) and both parents of 1. The
        // grandparent 4 is an ancestor of the common ancestor 3, so the
        // aunt/uncle line through 4 is not listed.
        let rows = [link(2, 4), link(3, 4), link(1, 2), link(1, 3)];
        assert_eq!(shape(&only(compute_kinship(&rows, &[], pid(1), pid(3)))), plain(1, 0));
    }

    #[test]
    fn siblings_whose_parents_are_cousins_are_listed_as_siblings_only() {
        // 10 -> (11, 12); 13 child of 11, 14 child of 12; 15 and 16 children of 13 + 14.
        let rows = [
            link(11, 10),
            link(12, 10),
            link(13, 11),
            link(14, 12),
            link(15, 13),
            link(15, 14),
            link(16, 13),
            link(16, 14),
        ];
        assert_eq!(shape(&only(compute_kinship(&rows, &[], pid(15), pid(16)))), plain(1, 1));
    }

    #[test]
    fn independent_in_law_route_is_listed_next_to_a_blood_relation_of_the_same_shape() {
        // Cousin(10) is my first cousin through UncleBob AND my spouse's
        // first cousin through the spouse's own family (21 -> 24 <- 25 <- 10).
        let mut rows = links();
        rows.extend([link(21, 24), link(25, 24), link(10, 25)]);
        let rels = compute_kinship(&rows, &parts(), pid(7), pid(10));
        assert_eq!(rels.len(), 2, "{rels:?}");
        assert_eq!(shape(&rels[0]), plain(2, 2));
        assert_eq!(shape(&rels[1]), (2, 2, false, false, Some(InLawSide::ViaOwnPartner)));
        assert_eq!(rels[1].path, path(&[7, 20, 21, 24, 25, 10]));
    }

    #[test]
    fn end_reason_without_a_date_counts_as_ended() {
        let undated = |reason| Partnership { end_reason: Some(reason), ..married(7, 20) };
        // Direct partners: reported, and not open.
        let divorced = [undated(PartnershipEndReason::Divorce)];
        let direct = only(compute_kinship(&links(), &divorced, pid(7), pid(20)));
        assert_eq!(direct.partnership.map(|p| p.is_open()), Some(false));
        // The undated divorce ends the in-law relation; an undated death does not.
        assert_unrelated(&compute_kinship(&links(), &divorced, pid(7), pid(21)));
        let widowed = [undated(PartnershipEndReason::Death)];
        let rel = only(compute_kinship(&links(), &widowed, pid(7), pid(21)));
        assert_eq!(rel.in_law, Some(InLawSide::ViaOwnPartner));
    }

    #[test]
    fn symmetry_is_an_exact_mirror() {
        let everyone =
            [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 13, 14, 15, 16, 17, 18, 20, 21, 22, 23, 30, 31];
        for x in everyone {
            for y in everyone {
                let forward = full(x, y);
                let backward = full(y, x);
                assert_eq!(forward.len(), backward.len(), "{x} vs {y}");
                for (f, b) in forward.iter().zip(&backward) {
                    assert_eq!((f.up, f.down, f.half, f.step), (b.down, b.up, b.half, b.step));
                    assert_eq!(f.in_law, b.in_law.map(InLawSide::flipped));
                    assert_eq!(f.partnership, b.partnership);
                    assert_eq!(f.path, b.path.iter().rev().copied().collect::<Vec<_>>());
                }
            }
        }
    }

    #[test]
    fn every_path_is_a_chain_of_real_edges() {
        let (links, parts) = (links(), parts());
        let is_edge = |x: PersonId, y: PersonId| {
            links
                .iter()
                .any(|l| (l.child_id, l.parent_id) == (x, y) || (l.child_id, l.parent_id) == (y, x))
                || parts.iter().any(|p| {
                    (p.partner_a_id, p.partner_b_id) == (x, y)
                        || (p.partner_a_id, p.partner_b_id) == (y, x)
                })
        };
        let everyone =
            [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 13, 14, 15, 16, 17, 18, 20, 21, 22, 23, 30, 31];
        for x in everyone {
            for y in everyone {
                for rel in full(x, y) {
                    assert_eq!(rel.path.first(), Some(&pid(x)));
                    assert_eq!(rel.path.last(), Some(&pid(y)));
                    let distinct: HashSet<_> = rel.path.iter().collect();
                    assert_eq!(
                        distinct.len(),
                        rel.path.len(),
                        "{x}->{y}: person twice in {:?}",
                        rel.path
                    );
                    // A named partnership row joins two adjacent path members.
                    if let Some(id) = rel.via_partnership {
                        let p = parts.iter().find(|p| p.id == id).unwrap();
                        assert!(rel.path.windows(2).any(|w| {
                            (w[0], w[1]) == (p.partner_a_id, p.partner_b_id)
                                || (w[0], w[1]) == (p.partner_b_id, p.partner_a_id)
                        }));
                    }
                    for pair in rel.path.windows(2) {
                        assert!(
                            is_edge(pair[0], pair[1]),
                            "{x}->{y}: {:?} broken at {pair:?}",
                            rel.path
                        );
                    }
                }
            }
        }
    }
}
