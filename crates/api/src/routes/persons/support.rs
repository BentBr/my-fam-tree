//! Shared helpers for the `/persons` handlers: validation caps, domain ↔
//! view mapping, PATCH merge, repo-error mapping and the link-consent gate.

use my_fam_tree_domain::{PersonDraft, PersonRepoError};
use uuid::Uuid;

use super::{PersonCreateReq, PersonUpdateReq, PersonView};
use crate::validation::{check_date_not_too_early, string_too_long};
use crate::{ApiError, FieldViolation};

pub(super) fn favourite_internal<E: std::fmt::Display>(e: E) -> ApiError {
    ApiError::Internal(anyhow::anyhow!(e.to_string()))
}

// Free-text length caps (security audit MEDIUM). Without these the only
// upper bound is actix's default 32 KB JSON body, which lets an admin
// store hundreds of 30 KB notes blobs per person. Caps are in CHARACTERS
// (Unicode scalar values via `chars().count()`), not bytes — a German
// umlaut and an emoji each count as one.
const NAME_MAX: u32 = 200;
const SHORT_MAX: u32 = 100;
const NOTES_MAX: u32 = 2000;

/// Bound a single string field's character count. `None` paths (empty
/// strings) pass; callers handle required-vs-optional separately.
fn check_max(path: &str, value: &str, max: u32) -> Result<(), ApiError> {
    if u32::try_from(value.chars().count()).unwrap_or(u32::MAX) > max {
        return Err(string_too_long(path, max));
    }
    Ok(())
}

pub(super) fn check_draft(d: &PersonDraft) -> Result<(), ApiError> {
    check_max("/given_name", &d.given_name, NAME_MAX)?;
    check_max("/family_name", &d.family_name, NAME_MAX)?;
    check_max("/name_at_birth", &d.name_at_birth, NAME_MAX)?;
    check_max("/nickname", &d.nickname, NAME_MAX)?;
    check_max("/gender", &d.gender, SHORT_MAX)?;
    check_max("/birth_place", &d.birth_place, SHORT_MAX)?;
    check_max("/notes", &d.notes, NOTES_MAX)?;
    check_date_not_too_early("/birth_date", d.birth_date)?;
    check_date_not_too_early("/death_date", d.death_date)?;
    Ok(())
}

fn internal<E: std::fmt::Display>(e: E) -> ApiError {
    ApiError::Internal(anyhow::anyhow!(e.to_string()))
}

/// Wall-clock TTL for the photo presigned URL. One hour balances "page
/// stays useful while the user reads" against "stale URL can't be reused
/// long-term if it leaks".
const PHOTO_URL_TTL: std::time::Duration = std::time::Duration::from_hours(1);

/// Resolve a stored `photo_key` to a fresh presigned URL. On storage
/// backend errors we log + return `None` so a single `MinIO` blip degrades
/// to "no photo shown" instead of a 500 on the whole person fetch.
///
/// Async because the S3 impl of `presigned_get` builds the URL via the
/// AWS SDK's `.presigned()` future. Calling it from a sync context inside
/// the actix arbiter panics ("can call blocking only when running on the
/// multi-threaded runtime").
async fn presigned_photo_url(
    object_store: &std::sync::Arc<dyn my_fam_tree_storage::ObjectStore>,
    key: Option<&str>,
) -> Option<String> {
    let key = key?;
    match object_store.presigned_get(key, PHOTO_URL_TTL).await {
        Ok(url) => Some(url),
        Err(e) => {
            tracing::warn!(error = ?e, photo_key = %key, "could not presign photo URL");
            None
        }
    }
}

pub(super) async fn to_view(
    p: my_fam_tree_domain::Person,
    object_store: &std::sync::Arc<dyn my_fam_tree_storage::ObjectStore>,
) -> PersonView {
    to_view_with_favourite(p, false, object_store).await
}

pub(super) async fn to_view_with_favourite(
    p: my_fam_tree_domain::Person,
    is_favourite_for_me: bool,
    object_store: &std::sync::Arc<dyn my_fam_tree_storage::ObjectStore>,
) -> PersonView {
    let photo_url = presigned_photo_url(object_store, p.photo_key.as_deref()).await;
    PersonView {
        id: p.id.into_uuid(),
        family_id: p.family_id.into_uuid(),
        given_name: p.given_name,
        family_name: p.family_name,
        name_at_birth: p.name_at_birth,
        nickname: p.nickname,
        gender: p.gender,
        birth_date: p.birth_date,
        birth_place: p.birth_place,
        death_date: p.death_date,
        notes: p.notes,
        linked_user_id: p.linked_user_id.map(my_fam_tree_domain::UserId::into_uuid),
        photo_url,
        is_favourite_for_me,
    }
}

pub(super) fn draft_from_create(req: PersonCreateReq) -> PersonDraft {
    PersonDraft {
        given_name: req.given_name,
        family_name: req.family_name,
        name_at_birth: req.name_at_birth,
        nickname: req.nickname,
        gender: req.gender,
        birth_date: req.birth_date,
        birth_place: req.birth_place,
        death_date: req.death_date,
        notes: req.notes,
        linked_user_id: req.linked_user_id.map(my_fam_tree_domain::UserId::from_uuid),
    }
}

/// Apply a partial `PersonUpdateReq` on top of an existing person, returning
/// the resulting draft. None fields preserve the existing column.
///
/// Date fields use the triple-state convention from
/// [`super::deserialize_optional_field`]:
///   - outer `None` → field absent in the body → preserve existing
///   - outer `Some(None)` → field present and JSON-null → clear (write NULL)
///   - outer `Some(Some(v))` → field present with a value → set to `v`
pub(super) fn merge_update(
    existing: &my_fam_tree_domain::Person,
    patch: PersonUpdateReq,
) -> PersonDraft {
    PersonDraft {
        given_name: patch.given_name.unwrap_or_else(|| existing.given_name.clone()),
        family_name: patch.family_name.unwrap_or_else(|| existing.family_name.clone()),
        name_at_birth: patch.name_at_birth.unwrap_or_else(|| existing.name_at_birth.clone()),
        nickname: patch.nickname.unwrap_or_else(|| existing.nickname.clone()),
        gender: patch.gender.unwrap_or_else(|| existing.gender.clone()),
        birth_date: patch.birth_date.unwrap_or(existing.birth_date),
        birth_place: patch.birth_place.unwrap_or_else(|| existing.birth_place.clone()),
        death_date: patch.death_date.unwrap_or(existing.death_date),
        notes: patch.notes.unwrap_or_else(|| existing.notes.clone()),
        linked_user_id: patch
            .linked_user_id
            .map(my_fam_tree_domain::UserId::from_uuid)
            .or(existing.linked_user_id),
    }
}

pub(super) fn map_person_repo_err(e: PersonRepoError, id: Option<Uuid>) -> ApiError {
    match e {
        PersonRepoError::NotFound => ApiError::PersonNotFound { id },
        PersonRepoError::LinkedUserConflict => ApiError::ConflictStale,
        PersonRepoError::Db(_) => internal(e),
    }
}

/// Gate caller-supplied `linked_user_id` on CREATE / PATCH against the
/// consent model.
///
/// Linking a person row to a user account is privacy-sensitive: the
/// linked user inherits the row's contacts, photos, "this is you"
/// highlight in the tree, and several FE affordances. Without a consent
/// check, any admin in the family could silently bind any person row
/// to any `user_id` (their own user, an unrelated user, an attacker's
/// throwaway, …) just by posting `linked_user_id: <uuid>`.
///
/// The dedicated consent paths are:
/// * `POST /persons/{id}/claim` — caller self-links (we gate on the
///   role; the caller is provably acting for themselves).
/// * The invite-email round-trip — for linking *other* users, where
///   the linked party must click their own mailbox before the bind
///   happens.
///
/// CREATE and PATCH stay permissive in three specific cases (so the
/// FE can keep working without contortions):
/// 1. `proposed` is `None` — the caller didn't touch the field (most
///    edits) or sent an explicit null (PATCH's `Option<Uuid>` shape
///    can't distinguish absent from null, but both mean "no change").
/// 2. `proposed == current` — defensive echo of the existing value
///    from a FE that re-sends every field on save.
/// 3. `proposed == caller_user_id` AND `current` is `None` — admin
///    self-link of an UNLINKED row (e.g., the family-owner's first
///    row representing themselves on CREATE; or an admin staking
///    claim to a still-orphan row on PATCH). Equivalent to
///    `POST /claim` after the fact; allowed inline for convenience.
///
///    The `current.is_none()` guard is load-bearing for the
///    REASSIGN case: without it, an admin could PATCH an already-
///    linked row to themselves, bypassing the linked user's
///    consent (they should have had to use the invite flow). With
///    the guard, reassign-to-self requires the original linked
///    user to first clear their own link (via `DELETE /claim`
///    or an explicit PATCH `linked_user_id: null` from their
///    session).
///
/// Anything else (proposed is some *other* user's id, OR proposed
/// is self but the row is already linked to someone else) is rejected
/// with `validation.link_consent_required` so the FE can show a
/// targeted message and the admin understands they need the invite
/// flow.
pub(super) fn check_link_consent(
    proposed: Option<my_fam_tree_domain::UserId>,
    current: Option<my_fam_tree_domain::UserId>,
    caller_user_id: my_fam_tree_domain::UserId,
) -> Result<(), ApiError> {
    match proposed {
        None => Ok(()),
        Some(p) if Some(p) == current => Ok(()),
        Some(p) if p == caller_user_id && current.is_none() => Ok(()),
        Some(_) => Err(ApiError::Validation(vec![FieldViolation::new(
            "/linked_user_id",
            "validation.link_consent_required",
            "Linking a person row to another user requires the invite-email \
             consent flow. Send an invite (POST /families/{id}/invites) and let \
             the recipient accept; or for self-link of an unlinked row, use \
             POST /persons/{id}/claim or set this field to your own user id.",
        )])),
    }
}

#[cfg(test)]
mod tests {
    use my_fam_tree_domain::UserId;
    use time::Date;
    use uuid::Uuid;

    use super::*;

    fn uid() -> UserId {
        UserId::from_uuid(Uuid::new_v4())
    }

    // The consent gate has a small but security-relevant truth table —
    // each branch gets its own test so a future refactor that drops a
    // branch fails loudly. Reviewers can read the test names as a spec.

    #[test]
    fn link_consent_allows_none_proposed() {
        // FE didn't touch the field (most edits) — always fine.
        let caller = uid();
        assert!(check_link_consent(None, None, caller).is_ok());
        assert!(check_link_consent(None, Some(uid()), caller).is_ok());
    }

    #[test]
    fn link_consent_allows_proposed_equal_to_current() {
        // Defensive echo — FE re-sends the existing value as part of a
        // wider edit. No change attempted, so the consent gate is moot.
        let caller = uid();
        let linked = uid();
        assert!(check_link_consent(Some(linked), Some(linked), caller).is_ok());
    }

    #[test]
    fn link_consent_allows_self_link_only_when_row_is_unlinked() {
        // Admin self-link at CREATE / PATCH of an UNLINKED row —
        // equivalent to POST /claim.
        let caller = uid();
        assert!(check_link_consent(Some(caller), None, caller).is_ok());
    }

    #[test]
    fn link_consent_rejects_admin_yanking_already_linked_row_to_self() {
        // The privacy bypass the consent gate is supposed to close:
        // an admin should NOT be able to silently take over a row
        // that's already linked to user A by PATCHing its
        // linked_user_id to themselves (PATCH would otherwise mimic
        // POST /claim without the original user's clearing step).
        // The `current.is_none()` guard makes this case fall through
        // to the rejection branch.
        let caller = uid();
        let other_user = uid();
        let err = check_link_consent(Some(caller), Some(other_user), caller)
            .expect_err("self-link over an existing link must require consent");
        match err {
            ApiError::Validation(v) => {
                assert_eq!(v.len(), 1);
                assert_eq!(v[0].path, "/linked_user_id");
                assert_eq!(v[0].code, "validation.link_consent_required");
            }
            _ => panic!("expected Validation, got {err:?}"),
        }
    }

    #[test]
    fn link_consent_rejects_proposed_equal_to_other_user() {
        // The consent hole: admin tries to link a row to some OTHER
        // user's id without an invite round-trip. Must be rejected.
        let caller = uid();
        let other = uid();
        let err = check_link_consent(Some(other), None, caller).expect_err("must reject");
        match err {
            ApiError::Validation(v) => {
                assert_eq!(v.len(), 1);
                assert_eq!(v[0].path, "/linked_user_id");
                assert_eq!(v[0].code, "validation.link_consent_required");
            }
            _ => panic!("expected Validation, got {err:?}"),
        }
    }

    #[test]
    fn link_consent_rejects_admin_moving_link_between_other_users() {
        // Admin tries to silently re-bind from user A to user B
        // (neither is the caller). Must be rejected even though
        // there's already a link there.
        let caller = uid();
        let user_a = uid();
        let user_b = uid();
        assert!(check_link_consent(Some(user_b), Some(user_a), caller).is_err());
    }

    // ---------------------------------------------------------------------
    // PATCH /persons/{id} triple-state for date fields. Each row of the
    // table in `deserialize_optional_field` doc gets its own test so a
    // future refactor that collapses absent/null can't silently re-break
    // the "uncheck deceased" UI flow.
    // ---------------------------------------------------------------------

    fn fixture_person_with_death_date(date: Option<Date>) -> my_fam_tree_domain::Person {
        my_fam_tree_domain::Person {
            id: my_fam_tree_domain::PersonId::from_uuid(Uuid::new_v4()),
            family_id: my_fam_tree_domain::FamilyId::from_uuid(Uuid::new_v4()),
            given_name: "Alice".into(),
            family_name: String::new(),
            name_at_birth: String::new(),
            nickname: String::new(),
            gender: String::new(),
            birth_date: None,
            birth_place: String::new(),
            death_date: date,
            notes: String::new(),
            linked_user_id: None,
            photo_key: None,
            created_at: time::macros::datetime!(2026-01-01 00:00 UTC),
            updated_at: time::macros::datetime!(2026-01-01 00:00 UTC),
        }
    }

    #[test]
    fn patch_with_absent_death_date_preserves_existing() {
        let existing_date = time::macros::date!(2020 - 01 - 01);
        let existing = fixture_person_with_death_date(Some(existing_date));
        // `{}` — field absent. `#[serde(default)]` produces `None`.
        let patch: PersonUpdateReq = serde_json::from_str(r#"{ "nickname": "X" }"#).unwrap();
        assert!(patch.death_date.is_none(), "absent field must deserialize to outer None");
        let draft = merge_update(&existing, patch);
        assert_eq!(draft.death_date, Some(existing_date), "absent must preserve");
    }

    #[test]
    fn patch_with_null_death_date_clears_existing() {
        // The split between absent and explicit-null is the whole point
        // of the triple-state pattern: a JSON `null` here deserializes
        // to outer `Some(None)`, which `merge_update` reads as "clear"
        // and writes NULL to the column.
        let existing_date = time::macros::date!(2020 - 01 - 01);
        let existing = fixture_person_with_death_date(Some(existing_date));
        let patch: PersonUpdateReq = serde_json::from_str(r#"{ "death_date": null }"#).unwrap();
        assert_eq!(patch.death_date, Some(None), "null must deserialize to outer Some(None)");
        let draft = merge_update(&existing, patch);
        assert_eq!(draft.death_date, None, "null must clear");
    }

    #[test]
    fn patch_with_value_death_date_sets_new_value() {
        let existing = fixture_person_with_death_date(None);
        let patch: PersonUpdateReq =
            serde_json::from_str(r#"{ "death_date": "2024-06-15" }"#).unwrap();
        let new_date = time::macros::date!(2024 - 06 - 15);
        assert_eq!(patch.death_date, Some(Some(new_date)), "value must deserialize to Some(Some)");
        let draft = merge_update(&existing, patch);
        assert_eq!(draft.death_date, Some(new_date), "value must set");
    }

    // Mirror the death_date trio for birth_date so the same shape is
    // pinned on both fields — they share the deserializer + merge path
    // and we don't want a future refactor to fix one and miss the other.

    #[test]
    fn patch_with_null_birth_date_clears_existing() {
        let mut existing = fixture_person_with_death_date(None);
        existing.birth_date = Some(time::macros::date!(1990 - 04 - 12));
        let patch: PersonUpdateReq = serde_json::from_str(r#"{ "birth_date": null }"#).unwrap();
        let draft = merge_update(&existing, patch);
        assert_eq!(draft.birth_date, None);
    }
}
