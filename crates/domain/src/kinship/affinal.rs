//! Relations that cross a partnership: direct partners, in-laws (exactly one
//! partner hop) and derived step relations. Every relation built here crosses
//! exactly ONE partnership row and records its id in `via_partnership`.

use std::collections::HashMap;

use uuid::Uuid;

use super::graph::{BloodRel, Graph, blood_relations};
use super::{InLawSide, Kinship, PartnerInfo};
use crate::{Partnership, PartnershipEndReason, PersonId};

/// Partners reachable through a QUALIFYING partnership — open, or ended by
/// death — each with the id of that partnership row. Divorce / separation end
/// the in-law and step relations. A row counts as ended as soon as it has an
/// end date OR an end reason (the API accepts a reason without a date).
#[derive(Debug)]
pub(super) struct HopPartners(HashMap<PersonId, Vec<(PersonId, Uuid)>>);

impl HopPartners {
    pub(super) fn new(partnerships: &[Partnership]) -> Self {
        let mut map: HashMap<PersonId, Vec<(PersonId, Uuid)>> = HashMap::new();
        let qualifying = partnerships.iter().filter(|p| {
            let open = p.ended_on.is_none() && p.end_reason.is_none();
            open || matches!(p.end_reason, Some(PartnershipEndReason::Death))
        });
        for p in qualifying {
            map.entry(p.partner_a_id).or_default().push((p.partner_b_id, p.id));
            map.entry(p.partner_b_id).or_default().push((p.partner_a_id, p.id));
        }
        Self(map)
    }

    fn of(&self, person: PersonId) -> &[(PersonId, Uuid)] {
        self.0.get(&person).map_or(&[], Vec::as_slice)
    }
}

/// Every partnership row between `a` and `b`, ended ones included.
pub(super) fn direct_partners(
    partnerships: &[Partnership],
    a: PersonId,
    b: PersonId,
) -> Vec<Kinship> {
    partnerships
        .iter()
        .filter(|p| {
            let pair = (p.partner_a_id, p.partner_b_id);
            pair == (a, b) || pair == (b, a)
        })
        .map(|p| Kinship {
            up: 0,
            down: 0,
            half: false,
            step: false,
            in_law: None,
            partnership: Some(PartnerInfo {
                kind: p.kind,
                ended_on: p.ended_on,
                end_reason: p.end_reason,
            }),
            via_partnership: Some(p.id),
            path: vec![a, b],
        })
        .collect()
}

const fn in_law(r: &BloodRel, side: InLawSide, via: Uuid, path: Vec<PersonId>) -> Kinship {
    Kinship {
        up: r.up,
        down: r.down,
        half: false,
        step: r.step,
        in_law: Some(side),
        partnership: None,
        via_partnership: Some(via),
        path,
    }
}

/// In-laws through exactly one partner hop.
///
/// Two shapes are NOT in-laws and are skipped here (the derived-step pass
/// owns the shallow cases): a descendant of A's partner (`up == 0`, a
/// step-child) and the partner of A's ancestor (`down == 0`, a step-parent).
/// An in-law relation is listed next to a blood relation of the same shape
/// when both hold; only a route that visits a person twice is dropped (by
/// the caller).
pub(super) fn in_laws(g: &Graph, hops: &HopPartners, a: PersonId, b: PersonId) -> Vec<Kinship> {
    let mut out = Vec::new();
    // B is a blood relative of A's partner.
    for &(hop, via) in hops.of(a).iter().filter(|(h, _)| *h != b) {
        for r in blood_relations(g, hop, b) {
            if r.up == 0 {
                continue;
            }
            let mut path = vec![a];
            path.extend(r.path.iter().copied());
            out.push(in_law(&r, InLawSide::ViaOwnPartner, via, path));
        }
    }
    // B is the partner of A's blood relative.
    for &(hop, via) in hops.of(b).iter().filter(|(h, _)| *h != a) {
        for r in blood_relations(g, a, hop) {
            if r.down == 0 {
                continue;
            }
            let mut path = r.path.clone();
            path.push(b);
            out.push(in_law(&r, InLawSide::ViaRelativesPartner, via, path));
        }
    }
    out
}

const fn step(up: u8, down: u8, via: Uuid, path: Vec<PersonId>) -> Kinship {
    Kinship {
        up,
        down,
        half: false,
        step: true,
        in_law: None,
        partnership: None,
        via_partnership: Some(via),
        path,
    }
}

/// `(parent, partnership id)` for every parent of `child` through whom
/// `candidate` is a derived step-parent: `candidate` is that parent's
/// qualifying partner and not a parent of `child` themselves.
fn step_parent_via(
    g: &Graph,
    hops: &HopPartners,
    child: PersonId,
    candidate: PersonId,
) -> Vec<(PersonId, Uuid)> {
    let parents = g.parents(child);
    if parents.contains(&candidate) {
        return Vec::new();
    }
    parents
        .into_iter()
        .flat_map(|parent| {
            hops.of(parent)
                .iter()
                .filter(move |(partner, _)| *partner == candidate)
                .map(move |(_, via)| (parent, *via))
        })
        .collect()
}

/// Derived step relations, deliberately shallow: step-parent `(1,0)`,
/// step-child `(0,1)` and step-sibling `(1,1)` — no step-grandparents.
pub(super) fn derived_step(
    g: &Graph,
    hops: &HopPartners,
    a: PersonId,
    b: PersonId,
) -> Vec<Kinship> {
    let mut out = Vec::new();
    for (parent, via) in step_parent_via(g, hops, a, b) {
        out.push(step(1, 0, via, vec![a, parent, b]));
    }
    for (parent, via) in step_parent_via(g, hops, b, a) {
        out.push(step(0, 1, via, vec![a, parent, b]));
    }
    // Step-siblings: a parent of A is partnered with a parent of B, and the
    // two share no parent (a shared parent makes them half/full siblings).
    let parents_a = g.parents(a);
    let parents_b = g.parents(b);
    if parents_a.iter().any(|p| parents_b.contains(p)) {
        return out;
    }
    for &pa in &parents_a {
        for &(pb, via) in hops.of(pa).iter().filter(|(p, _)| parents_b.contains(p)) {
            out.push(step(1, 1, via, vec![a, pa, pb, b]));
        }
    }
    out
}
