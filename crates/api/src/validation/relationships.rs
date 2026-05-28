//! Cross-aggregate relationship validation.
//!
//! These validators run *after* the route has parsed the request and *before*
//! the persistence write. They consume a snapshot of the family's current
//! graph — `persons`, `parent_links`, and (for partnership rules)
//! `partnerships` — and return either:
//!
//!   - `Err(ApiError::Validation(...))` for **hard** rules. The HTTP layer
//!     surfaces this as 422 with the offending field paths.
//!   - `Ok(Vec<Warning>)` for **soft** rules. Warnings are attached to
//!     `meta.warnings` on the success envelope; the write still proceeds.
//!
//! The functions here are pure (no I/O, no async) so they can be unit-tested
//! with synthetic graphs. The DB cycle trigger (Phase 5 / Task 12a, brought
//! forward in this commit) is the race-safe backstop for the cycle rule;
//! the in-memory `would_create_cycle` check stays in the route layer as a
//! fast-path that returns before any DB round-trip on the common case.

use std::collections::{HashMap, HashSet};

use chrono::{Duration, NaiveDate};
use my_family_domain::{
    ParentKind, ParentLink, Partnership, PartnershipEndReason, Person, PersonId,
};

use crate::response::Warning;
use crate::{ApiError, FieldViolation};

/// Approximate length of a full-term pregnancy. Used to reject biological
/// parents whose recorded death date precedes the child's birth by more
/// than ~9 months.
const GESTATION_DAYS: i64 = 280;

/// Minimum age difference (in days) we expect between a biological parent
/// and child. 14 years is the warning threshold — below this we don't
/// reject the edge, just flag it for human review. Calendar days rather
/// than calendar years keeps the helper pure (no leap-year arithmetic).
const MIN_PARENT_CHILD_GAP_DAYS: i64 = 14 * 365;

/// Validate a candidate `(child, parent)` edge against the family's
/// existing graph. Returns soft warnings on the success path.
///
/// Inputs:
///   - `child_id`, `parent_id`, `kind`: the candidate edge.
///   - `persons`: all persons in the family. Need not contain `child_id`
///     or `parent_id`; missing persons are treated as having no known
///     birth/death dates and no rule fires for them.
///   - `existing_links`: all current parent-link rows for the family.
///     Used for the "≤ 2 biological parents" cap.
///
/// # Errors
/// Returns `ApiError::Validation` when a hard rule is violated.
pub fn check_parent_link(
    child_id: PersonId,
    parent_id: PersonId,
    kind: ParentKind,
    persons: &[Person],
    existing_links: &[ParentLink],
) -> Result<Vec<Warning>, ApiError> {
    let by_id = persons_by_id(persons);
    let child = by_id.get(&child_id);
    let parent = by_id.get(&parent_id);

    let mut violations: Vec<FieldViolation> = Vec::new();
    let mut warnings: Vec<Warning> = Vec::new();

    // Hard rule 1: parent born strictly before child.
    if let (Some(p), Some(c)) = (parent, child)
        && let (Some(pb), Some(cb)) = (p.birth_date, c.birth_date)
        && pb >= cb
    {
        violations.push(FieldViolation::new(
            "/parent_id",
            "validation.parent_not_older_than_child",
            "parent must be born strictly before the child",
        ));
    }

    // Hard rule 2: biological parent alive at conception.
    if kind == ParentKind::Biological
        && let (Some(p), Some(c)) = (parent, child)
        && let (Some(death), Some(birth)) = (p.death_date, c.birth_date)
        && (birth - death) >= Duration::days(GESTATION_DAYS)
    {
        violations.push(FieldViolation::new(
            "/parent_id",
            "validation.parent_deceased_before_child",
            "biological parent died more than nine months before the child was born",
        ));
    }

    // Hard rule 3: at most two biological parents per child.
    if kind == ParentKind::Biological {
        let bio_count = existing_links
            .iter()
            .filter(|l| l.child_id == child_id && l.kind == ParentKind::Biological)
            .filter(|l| l.parent_id != parent_id) // tolerate upsert of the same edge
            .count();
        if bio_count >= 2 {
            violations.push(FieldViolation::new(
                "/parent_id",
                "validation.too_many_biological_parents",
                "a child can have at most two biological parents",
            ));
        }
    }

    if !violations.is_empty() {
        return Err(ApiError::Validation(violations));
    }

    // Soft rule 5: parent-child age gap < 14 years.
    if let (Some(p), Some(c)) = (parent, child)
        && let (Some(pb), Some(cb)) = (p.birth_date, c.birth_date)
        && pb < cb
        && (cb - pb) < Duration::days(MIN_PARENT_CHILD_GAP_DAYS)
    {
        warnings.push(Warning {
            code: "warning.parent_child_age_gap_under_14y".to_string(),
            message: "parent-child age gap is under fourteen years".to_string(),
            path: Some("/parent_id".to_string()),
        });
    }

    Ok(warnings)
}

/// Validate a candidate partnership against the family's existing graph.
/// Returns soft warnings on the success path.
///
/// `ended_on` and `end_reason` are accepted because we may create a
/// partnership that's already historical (e.g. import flows). The
/// `end_reason == Death` cross-check (rule 7) only fires when those are
/// `Some`.
///
/// # Errors
/// Returns `ApiError::Validation` when a hard rule is violated.
#[allow(
    clippy::too_many_arguments,
    clippy::similar_names,
    reason = "validator carries the full candidate row + graph snapshot; partner_a/partner_b are the canonical names for the two halves of the pair"
)]
pub fn check_partnership(
    partner_a_id: PersonId,
    partner_b_id: PersonId,
    started_on: Option<NaiveDate>,
    ended_on: Option<NaiveDate>,
    end_reason: Option<PartnershipEndReason>,
    persons: &[Person],
    parent_links: &[ParentLink],
    _existing_partnerships: &[Partnership],
) -> Result<Vec<Warning>, ApiError> {
    let by_id = persons_by_id(persons);
    let pa = by_id.get(&partner_a_id);
    let pb = by_id.get(&partner_b_id);

    let mut violations: Vec<FieldViolation> = Vec::new();
    let mut warnings: Vec<Warning> = Vec::new();

    // Hard rule 4: partnership starts after both partners' births.
    if let Some(s) = started_on {
        for partner in [pa, pb].into_iter().flatten() {
            if let Some(b) = partner.birth_date
                && s < b
            {
                violations.push(FieldViolation::new(
                    "/started_on",
                    "validation.partnership_before_birth",
                    "partnership cannot start before a partner's birth",
                ));
                break; // one violation on /started_on is enough
            }
        }
    }

    if !violations.is_empty() {
        return Err(ApiError::Validation(violations));
    }

    // Soft rule 6: sibling partnership (shared parent).
    let parents_a = parents_of(partner_a_id, parent_links);
    let parents_b = parents_of(partner_b_id, parent_links);
    if !parents_a.is_disjoint(&parents_b) {
        warnings.push(Warning {
            code: "warning.sibling_partnership".to_string(),
            message: "partners share at least one parent".to_string(),
            path: None,
        });
    }

    // Soft rule 7: end_reason == 'death' but no partner death matches ended_on.
    if end_reason == Some(PartnershipEndReason::Death) {
        let matches = ended_on.is_some_and(|ended| {
            [pa, pb].into_iter().flatten().any(|p| p.death_date == Some(ended))
        });
        if !matches {
            warnings.push(Warning {
                code: "warning.end_reason_death_mismatch".to_string(),
                message: "end_reason is 'death' but no partner has a matching death_date"
                    .to_string(),
                path: Some("/end_reason".to_string()),
            });
        }
    }

    Ok(warnings)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn persons_by_id(persons: &[Person]) -> HashMap<PersonId, &Person> {
    persons.iter().map(|p| (p.id, p)).collect()
}

fn parents_of(person: PersonId, links: &[ParentLink]) -> HashSet<PersonId> {
    links.iter().filter(|l| l.child_id == person).map(|l| l.parent_id).collect()
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "test code: assertion helpers may panic on unexpected variants"
)]
mod tests {
    use chrono::{DateTime, NaiveDate, Utc};
    use my_family_domain::{FamilyId, Person, PersonId};
    use uuid::Uuid;

    use super::*;

    fn pid(n: u8) -> PersonId {
        let mut bytes = [0_u8; 16];
        bytes[15] = n;
        PersonId::from_uuid(Uuid::from_bytes(bytes))
    }

    fn person(id: PersonId, birth: Option<NaiveDate>, death: Option<NaiveDate>) -> Person {
        Person {
            id,
            family_id: FamilyId::from_uuid(Uuid::nil()),
            given_name: String::new(),
            family_name: String::new(),
            name_at_birth: String::new(),
            nickname: String::new(),
            gender: String::new(),
            birth_date: birth,
            birth_place: String::new(),
            death_date: death,
            notes: String::new(),
            linked_user_id: None,
            photo_key: None,
            created_at: DateTime::<Utc>::from_timestamp(0, 0).unwrap(),
            updated_at: DateTime::<Utc>::from_timestamp(0, 0).unwrap(),
        }
    }

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    fn link(child: PersonId, parent: PersonId, kind: ParentKind) -> ParentLink {
        ParentLink { child_id: child, parent_id: parent, kind, note: String::new() }
    }

    // -----------------------------------------------------------------
    // Parent link — happy path
    // -----------------------------------------------------------------

    #[test]
    fn parent_link_happy_path_returns_no_warnings() {
        let child = pid(2);
        let parent = pid(1);
        let persons = vec![
            person(parent, Some(date(1960, 1, 1)), None),
            person(child, Some(date(1990, 1, 1)), None),
        ];
        let warnings =
            check_parent_link(child, parent, ParentKind::Biological, &persons, &[]).unwrap();
        assert!(warnings.is_empty(), "unexpected warnings: {warnings:?}");
    }

    // -----------------------------------------------------------------
    // Rule 1: parent_not_older_than_child
    // -----------------------------------------------------------------

    #[test]
    fn rule1_rejects_parent_born_same_day_as_child() {
        let child = pid(2);
        let parent = pid(1);
        let persons = vec![
            person(parent, Some(date(1990, 1, 1)), None),
            person(child, Some(date(1990, 1, 1)), None),
        ];
        let err = check_parent_link(child, parent, ParentKind::Biological, &persons, &[])
            .expect_err("must reject");
        match err {
            ApiError::Validation(v) => {
                assert!(v.iter().any(|f| f.code == "validation.parent_not_older_than_child"));
                assert_eq!(v[0].path, "/parent_id");
            }
            other => panic!("expected Validation, got {other:?}"),
        }
    }

    #[test]
    fn rule1_rejects_parent_born_after_child() {
        let child = pid(2);
        let parent = pid(1);
        let persons = vec![
            person(parent, Some(date(2000, 6, 1)), None),
            person(child, Some(date(1990, 1, 1)), None),
        ];
        let err = check_parent_link(child, parent, ParentKind::Legal, &persons, &[])
            .expect_err("must reject");
        assert!(matches!(err, ApiError::Validation(_)));
    }

    // -----------------------------------------------------------------
    // Rule 2: parent_deceased_before_child
    // -----------------------------------------------------------------

    #[test]
    fn rule2_rejects_biological_parent_dead_more_than_9mo_before_birth() {
        let child = pid(2);
        let parent = pid(1);
        // Parent died Jan 1 2000, child born Jan 1 2001 (~12 months later).
        let persons = vec![
            person(parent, Some(date(1960, 1, 1)), Some(date(2000, 1, 1))),
            person(child, Some(date(2001, 1, 1)), None),
        ];
        let err = check_parent_link(child, parent, ParentKind::Biological, &persons, &[])
            .expect_err("must reject");
        match err {
            ApiError::Validation(v) => {
                assert!(v.iter().any(|f| f.code == "validation.parent_deceased_before_child"));
            }
            other => panic!("expected Validation, got {other:?}"),
        }
    }

    #[test]
    fn rule2_allows_biological_parent_dying_during_pregnancy() {
        let child = pid(2);
        let parent = pid(1);
        // Parent died 200 days before birth - within gestation window.
        let persons = vec![
            person(parent, Some(date(1960, 1, 1)), Some(date(1989, 6, 15))),
            person(child, Some(date(1990, 1, 1)), None),
        ];
        let warnings =
            check_parent_link(child, parent, ParentKind::Biological, &persons, &[]).unwrap();
        // Could still trigger rule 5 if gap < 14y; but 1960→1990 is 30y so no warning.
        assert!(warnings.is_empty(), "unexpected warnings: {warnings:?}");
    }

    #[test]
    fn rule2_does_not_apply_to_non_biological_parents() {
        let child = pid(2);
        let parent = pid(1);
        // Same setup as the rejecting case, but kind=Adoptive.
        let persons = vec![
            person(parent, Some(date(1960, 1, 1)), Some(date(2000, 1, 1))),
            person(child, Some(date(2001, 1, 1)), None),
        ];
        let warnings = check_parent_link(child, parent, ParentKind::Adoptive, &persons, &[])
            .expect("adoptive must not be blocked by rule 2");
        // Rule 5 may fire here (41y gap → no), so empty.
        assert!(warnings.is_empty());
    }

    // -----------------------------------------------------------------
    // Rule 3: too_many_biological_parents
    // -----------------------------------------------------------------

    #[test]
    fn rule3_rejects_third_biological_parent() {
        let child = pid(10);
        let p1 = pid(1);
        let p2 = pid(2);
        let p3 = pid(3);
        let persons = vec![
            person(p1, Some(date(1960, 1, 1)), None),
            person(p2, Some(date(1962, 1, 1)), None),
            person(p3, Some(date(1964, 1, 1)), None),
            person(child, Some(date(1990, 1, 1)), None),
        ];
        let existing =
            vec![link(child, p1, ParentKind::Biological), link(child, p2, ParentKind::Biological)];
        let err = check_parent_link(child, p3, ParentKind::Biological, &persons, &existing)
            .expect_err("third biological must be rejected");
        match err {
            ApiError::Validation(v) => {
                assert!(v.iter().any(|f| f.code == "validation.too_many_biological_parents"));
            }
            other => panic!("expected Validation, got {other:?}"),
        }
    }

    #[test]
    fn rule3_allows_unbounded_non_biological_parents() {
        let child = pid(10);
        let p1 = pid(1);
        let p2 = pid(2);
        let p3 = pid(3);
        let p4 = pid(4);
        let persons = vec![
            person(p1, Some(date(1960, 1, 1)), None),
            person(p2, Some(date(1962, 1, 1)), None),
            person(p3, Some(date(1964, 1, 1)), None),
            person(p4, Some(date(1966, 1, 1)), None),
            person(child, Some(date(1990, 1, 1)), None),
        ];
        // Two bio + one step already exist; adding a fourth (adoptive) is fine.
        let existing = vec![
            link(child, p1, ParentKind::Biological),
            link(child, p2, ParentKind::Biological),
            link(child, p3, ParentKind::Step),
        ];
        let warnings =
            check_parent_link(child, p4, ParentKind::Adoptive, &persons, &existing).unwrap();
        assert!(warnings.is_empty());
    }

    #[test]
    fn rule3_upsert_of_same_biological_edge_is_allowed() {
        let child = pid(10);
        let p1 = pid(1);
        let p2 = pid(2);
        let persons = vec![
            person(p1, Some(date(1960, 1, 1)), None),
            person(p2, Some(date(1962, 1, 1)), None),
            person(child, Some(date(1990, 1, 1)), None),
        ];
        let existing =
            vec![link(child, p1, ParentKind::Biological), link(child, p2, ParentKind::Biological)];
        // Re-inserting p1 as biological is an upsert, not a third parent.
        let warnings =
            check_parent_link(child, p1, ParentKind::Biological, &persons, &existing).unwrap();
        assert!(warnings.is_empty());
    }

    // -----------------------------------------------------------------
    // Rule 5: warning.parent_child_age_gap_under_14y
    // -----------------------------------------------------------------

    #[test]
    fn rule5_warns_when_gap_under_14_years() {
        let child = pid(2);
        let parent = pid(1);
        // 10 year gap.
        let persons = vec![
            person(parent, Some(date(1980, 1, 1)), None),
            person(child, Some(date(1990, 1, 1)), None),
        ];
        let warnings =
            check_parent_link(child, parent, ParentKind::Biological, &persons, &[]).unwrap();
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].code, "warning.parent_child_age_gap_under_14y");
    }

    #[test]
    fn rule5_no_warn_when_gap_is_exactly_14_years() {
        let child = pid(2);
        let parent = pid(1);
        // 14 * 365 = 5110 days; pick dates exactly that far apart.
        let parent_dob = date(1976, 1, 1);
        let child_dob = parent_dob + Duration::days(14 * 365);
        let persons =
            vec![person(parent, Some(parent_dob), None), person(child, Some(child_dob), None)];
        let warnings =
            check_parent_link(child, parent, ParentKind::Biological, &persons, &[]).unwrap();
        assert!(warnings.is_empty(), "boundary should not warn: {warnings:?}");
    }

    // -----------------------------------------------------------------
    // Partnership rules
    // -----------------------------------------------------------------

    #[test]
    fn partnership_happy_path_returns_no_warnings() {
        let a = pid(1);
        let b = pid(2);
        let persons =
            vec![person(a, Some(date(1980, 1, 1)), None), person(b, Some(date(1982, 1, 1)), None)];
        let warnings =
            check_partnership(a, b, Some(date(2005, 6, 1)), None, None, &persons, &[], &[])
                .unwrap();
        assert!(warnings.is_empty());
    }

    #[test]
    fn rule4_rejects_partnership_before_either_birth() {
        let a = pid(1);
        let b = pid(2);
        let persons =
            vec![person(a, Some(date(1980, 1, 1)), None), person(b, Some(date(1985, 1, 1)), None)];
        let err = check_partnership(a, b, Some(date(1984, 1, 1)), None, None, &persons, &[], &[])
            .expect_err("must reject");
        match err {
            ApiError::Validation(v) => {
                assert_eq!(v[0].code, "validation.partnership_before_birth");
                assert_eq!(v[0].path, "/started_on");
            }
            other => panic!("expected Validation, got {other:?}"),
        }
    }

    #[test]
    fn rule4_allows_partnership_on_exact_birthday() {
        let a = pid(1);
        let b = pid(2);
        let persons =
            vec![person(a, Some(date(1980, 1, 1)), None), person(b, Some(date(1985, 1, 1)), None)];
        // started_on == later partner's birth_date is allowed by the spec
        // (`s >= b`).
        let warnings =
            check_partnership(a, b, Some(date(1985, 1, 1)), None, None, &persons, &[], &[])
                .unwrap();
        assert!(warnings.is_empty());
    }

    #[test]
    fn rule6_warns_when_partners_share_a_parent() {
        let a = pid(1);
        let b = pid(2);
        let dad = pid(9);
        let persons = vec![
            person(a, Some(date(1980, 1, 1)), None),
            person(b, Some(date(1982, 1, 1)), None),
            person(dad, Some(date(1950, 1, 1)), None),
        ];
        let links =
            vec![link(a, dad, ParentKind::Biological), link(b, dad, ParentKind::Biological)];
        let warnings = check_partnership(a, b, None, None, None, &persons, &links, &[]).unwrap();
        assert!(warnings.iter().any(|w| w.code == "warning.sibling_partnership"));
    }

    #[test]
    fn rule7_warns_when_death_end_reason_lacks_matching_death_date() {
        let a = pid(1);
        let b = pid(2);
        let persons =
            vec![person(a, Some(date(1980, 1, 1)), None), person(b, Some(date(1982, 1, 1)), None)];
        let warnings = check_partnership(
            a,
            b,
            Some(date(2005, 6, 1)),
            Some(date(2020, 3, 1)),
            Some(PartnershipEndReason::Death),
            &persons,
            &[],
            &[],
        )
        .unwrap();
        assert!(warnings.iter().any(|w| w.code == "warning.end_reason_death_mismatch"));
    }

    #[test]
    fn rule7_no_warn_when_death_end_reason_matches_partner_death_date() {
        let a = pid(1);
        let b = pid(2);
        let persons = vec![
            person(a, Some(date(1980, 1, 1)), Some(date(2020, 3, 1))),
            person(b, Some(date(1982, 1, 1)), None),
        ];
        let warnings = check_partnership(
            a,
            b,
            Some(date(2005, 6, 1)),
            Some(date(2020, 3, 1)),
            Some(PartnershipEndReason::Death),
            &persons,
            &[],
            &[],
        )
        .unwrap();
        assert!(!warnings.iter().any(|w| w.code == "warning.end_reason_death_mismatch"));
    }
}
