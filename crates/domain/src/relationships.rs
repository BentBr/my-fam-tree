//! Pure relationship logic: cycle prevention and partner-pair canonicalization.
//!
//! The cycle check is intentionally in-memory and trait-free so it can be
//! unit-tested without a database. Persistence implementations must still wrap
//! the check + insert in a single transaction (SERIALIZABLE) to close the
//! TOCTOU window under concurrent writes.

use std::collections::{HashMap, HashSet};

use crate::PersonId;

/// Returns `true` if adding `(new_child, new_parent)` to `edges` would create
/// a cycle.
///
/// `edges` is the list of existing `(child, parent)` rows. A cycle exists when
/// `new_child` is reachable by walking parents-of-parents starting from
/// `new_parent`.
#[must_use]
pub fn would_create_cycle(
    edges: &[(PersonId, PersonId)],
    new_child: PersonId,
    new_parent: PersonId,
) -> bool {
    if new_child == new_parent {
        return true;
    }
    let mut parents_of: HashMap<PersonId, Vec<PersonId>> = HashMap::new();
    for (c, p) in edges {
        parents_of.entry(*c).or_default().push(*p);
    }
    let mut stack = vec![new_parent];
    let mut visited = HashSet::new();
    while let Some(node) = stack.pop() {
        if node == new_child {
            return true;
        }
        if !visited.insert(node) {
            continue;
        }
        if let Some(parents) = parents_of.get(&node) {
            for p in parents {
                stack.push(*p);
            }
        }
    }
    false
}

/// Returns the pair in canonical `(min, max)` order so the DB
/// `CHECK (partner_a_id < partner_b_id)` always holds. `None` when `a == b`.
#[must_use]
pub fn canonicalize_pair(a: PersonId, b: PersonId) -> Option<(PersonId, PersonId)> {
    if a == b {
        return None;
    }
    if a.into_uuid() < b.into_uuid() { Some((a, b)) } else { Some((b, a)) }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use uuid::Uuid;

    use super::*;

    fn pid(n: u8) -> PersonId {
        let mut bytes = [0u8; 16];
        bytes[15] = n;
        PersonId::from_uuid(Uuid::from_bytes(bytes))
    }

    #[test]
    fn self_parent_is_cycle() {
        assert!(would_create_cycle(&[], pid(1), pid(1)));
    }

    #[test]
    fn direct_cycle_is_detected() {
        // 1 is parent of 2, so 2 cannot become parent of 1.
        assert!(would_create_cycle(&[(pid(2), pid(1))], pid(1), pid(2)));
    }

    #[test]
    fn deep_cycle_is_detected() {
        // 1 -> 2 -> 3 (each is parent of the prev). Adding 3 as parent of 1 closes the loop.
        let edges = vec![(pid(1), pid(2)), (pid(2), pid(3))];
        assert!(would_create_cycle(&edges, pid(3), pid(1)));
    }

    #[test]
    fn unrelated_parents_are_fine() {
        let edges = vec![(pid(1), pid(2))];
        assert!(!would_create_cycle(&edges, pid(1), pid(3)));
    }

    #[test]
    fn canonicalize_orders_pair() {
        let (a, b) = canonicalize_pair(pid(2), pid(1)).expect("distinct pair");
        assert!(a.into_uuid() < b.into_uuid());
        assert_eq!(canonicalize_pair(pid(1), pid(1)), None);
    }
}
