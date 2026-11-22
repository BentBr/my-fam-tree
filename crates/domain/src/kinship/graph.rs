//! Parent-DAG traversal for kinship: ancestor maps and lowest-common-ancestor
//! blood relations. Pure, no I/O; the DAG property is guaranteed by the
//! insert-time cycle check (`crate::relationships::would_create_cycle`).

use std::collections::{HashMap, HashSet};

use crate::{ParentKind, ParentLink, PersonId};

/// Depth cap for ancestor walks: a common ancestor more than 32 generations
/// above either person is not found. Real families never get close; this
/// guards against pathological data.
const MAX_GENERATIONS: u8 = 32;

/// `child -> [(parent, via_step_link)]`, in input order.
#[derive(Debug)]
pub(super) struct Graph {
    parents_of: HashMap<PersonId, Vec<(PersonId, bool)>>,
}

impl Graph {
    pub(super) fn new(links: &[ParentLink]) -> Self {
        let mut parents_of: HashMap<PersonId, Vec<(PersonId, bool)>> = HashMap::new();
        for l in links {
            let is_step = matches!(l.kind, ParentKind::Step);
            parents_of.entry(l.child_id).or_default().push((l.parent_id, is_step));
        }
        Self { parents_of }
    }

    /// Recorded parents of `child` (any link kind), in input order.
    pub(super) fn parents(&self, child: PersonId) -> Vec<PersonId> {
        self.parents_of.get(&child).into_iter().flatten().map(|(p, _)| *p).collect()
    }

    /// Parents of `child` linked through a `step` link (`true`) or through
    /// any other kind (`false`).
    fn parents_by_step(&self, child: PersonId, step: bool) -> Vec<PersonId> {
        let links = self.parents_of.get(&child).into_iter().flatten();
        links.filter(|(_, is_step)| *is_step == step).map(|(p, _)| *p).collect()
    }
}

#[derive(Debug)]
struct AncEntry {
    depth: u8,
    step: bool,
    /// `[start, …, ancestor]` along the chosen shortest walk.
    path: Vec<PersonId>,
}

impl AncEntry {
    /// Order among walks of EQUAL length to one ancestor: blood before step,
    /// then by path ids. Never depends on the order of the input rows.
    fn beats(&self, other: &Self) -> bool {
        self.step
            .cmp(&other.step)
            .then_with(|| {
                self.path
                    .iter()
                    .map(PersonId::as_uuid)
                    .cmp(other.path.iter().map(PersonId::as_uuid))
            })
            .is_lt()
    }
}

/// Ancestors of `start` (itself included at depth 0), one generation at a
/// time, at most `MAX_GENERATIONS` up. Each ancestor keeps ONE walk: the
/// shortest, and among equally short ones a blood walk before one that
/// crosses a step link (see `AncEntry::beats`).
fn ancestors_of(g: &Graph, start: PersonId) -> HashMap<PersonId, AncEntry> {
    let mut out = HashMap::from([(start, AncEntry { depth: 0, step: false, path: vec![start] })]);
    let mut frontier = vec![start];
    for depth in 1..=MAX_GENERATIONS {
        let mut next: HashMap<PersonId, AncEntry> = HashMap::new();
        for cur in &frontier {
            let Some(entry) = out.get(cur) else { continue };
            for (parent, is_step) in g.parents_of.get(cur).into_iter().flatten() {
                if out.contains_key(parent) {
                    continue;
                }
                let mut path = entry.path.clone();
                path.push(*parent);
                let candidate = AncEntry { depth, step: entry.step || *is_step, path };
                if next.get(parent).is_none_or(|best| candidate.beats(best)) {
                    next.insert(*parent, candidate);
                }
            }
        }
        if next.is_empty() {
            break;
        }
        frontier = next.keys().copied().collect();
        out.extend(next);
    }
    out
}

#[derive(Debug)]
pub(super) struct BloodRel {
    pub(super) up: u8,
    pub(super) down: u8,
    pub(super) half: bool,
    pub(super) step: bool,
    /// `[x, …, common ancestor, …, y]`.
    pub(super) path: Vec<PersonId>,
}

/// Half-siblings only when that is KNOWN: besides the shared parent, each
/// has a recorded parent the other positively lacks. Step links never count
/// as a parent of one's own, but they do say who is NOT a parent: when the
/// other's parent is my step-parent, my other parent is known to differ. A
/// sibling whose second parent is simply not entered yet stays a full sibling.
fn known_half_siblings(g: &Graph, x: PersonId, y: PersonId) -> bool {
    let (px, py) = (g.parents_by_step(x, false), g.parents_by_step(y, false));
    // `mine` holds a parent the other lacks, or one of `theirs` is ruled out
    // for me because that person is my step-parent.
    let differs = |mine: &[PersonId], theirs: &[PersonId], my_step: &[PersonId]| {
        mine.iter().any(|p| !theirs.contains(p)) || theirs.iter().any(|p| my_step.contains(p))
    };
    differs(&px, &py, &g.parents_by_step(x, true)) && differs(&py, &px, &g.parents_by_step(y, true))
}

/// All blood relations between `x` and `y` via LOWEST common ancestors.
///
/// A common ancestor is dropped when it is a proper ancestor of another
/// common ancestor (a grandparent adds nothing beyond the parent). This is
/// deliberate even where the dropped ancestor is reached on an independent
/// line: a parent who is also the other parent's sibling is listed as parent
/// only, siblings whose parents are cousins as siblings only. Ancestor
/// couples collapse by grouping on `(up, down, step)`; a `(1,1)` backed by
/// exactly one shared parent is flagged `half` when both siblings also have a
/// parent of their own (see `known_half_siblings`). A step relation is dropped when a
/// blood relation of the same `(up, down)` exists.
///
/// Deterministic and direction-symmetric: surviving ancestors are visited in
/// id order, so a couple always yields the path through the same partner.
pub(super) fn blood_relations(g: &Graph, x: PersonId, y: PersonId) -> Vec<BloodRel> {
    if x == y {
        return Vec::new();
    }
    let anc_x = ancestors_of(g, x);
    let anc_y = ancestors_of(g, y);
    let common: HashSet<PersonId> =
        anc_x.keys().filter(|k| anc_y.contains_key(*k)).copied().collect();
    let mut redundant: HashSet<PersonId> = HashSet::new();
    for c in &common {
        for (anc, entry) in ancestors_of(g, *c) {
            if entry.depth > 0 && common.contains(&anc) {
                redundant.insert(anc);
            }
        }
    }
    let mut lowest: Vec<PersonId> = common.difference(&redundant).copied().collect();
    lowest.sort_unstable_by_key(|p| p.into_uuid());

    let mut rels: Vec<BloodRel> = Vec::new();
    for c in lowest {
        let (Some(ex), Some(ey)) = (anc_x.get(&c), anc_y.get(&c)) else { continue };
        let (up, down, step) = (ex.depth, ey.depth, ex.step || ey.step);
        if let Some(same) = rels.iter_mut().find(|r| (r.up, r.down, r.step) == (up, down, step)) {
            // A second ancestor with the same shape: an ancestor couple, so
            // siblings are full siblings. The first (lowest-id) path stays.
            same.half = false;
            continue;
        }
        let mut path = ex.path.clone(); // [x … c]
        path.extend(ey.path.iter().rev().skip(1)); // … y]
        let half = up == 1 && down == 1 && !step && known_half_siblings(g, x, y);
        rels.push(BloodRel { up, down, half, step, path });
    }
    // A relation that only exists through a step link is redundant next to a
    // blood relation of the same shape (an ancestor couple where one partner
    // is a step-parent on the way up): keep the blood one.
    let blood: HashSet<(u8, u8)> =
        rels.iter().filter(|r| !r.step).map(|r| (r.up, r.down)).collect();
    rels.retain(|r| !r.step || !blood.contains(&(r.up, r.down)));
    rels
}
