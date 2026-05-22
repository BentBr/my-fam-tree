//! Server-side rules for the person contact fields (`email`, `phone`, postal
//! address). Split out of `persons.rs` to keep that file inside the 500-line
//! cap while still living next to the route module that owns the policy.
//!
//! Two concerns:
//! - [`validate_contact_fields`] applies the shared per-field length cap and
//!   the email-syntax check (skipped when the linked-user sync path will
//!   overwrite it).
//! - [`sync_email_from_linked_user`] resolves `users.email` for a draft with
//!   `linked_user_id` set and writes the column from that lookup — the rule
//!   that makes the read path (`GET /persons/{id}`, tree payload) cheap.

use my_family_domain::PersonDraft;

use crate::validation::{email_invalid, looks_like_email, string_too_long};
use crate::{ApiError, AppState};

/// Cap free-form contact strings so the column doesn't get used as a notes
/// dump. Picked at 128 chars — plenty for a phone number or a German postal
/// address line ("Friedrich-Ebert-Allee 25b") while still small enough that
/// indexing + SVG-tooltip rendering stay cheap. `email` re-uses the same cap;
/// every contact field surfaces a `validation.string_too_long` violation when
/// the user exceeds it.
pub const CONTACT_FIELD_MAX_LEN: usize = 128;

/// Map any error from the user repo into our sanitized internal error
/// envelope. Cross-module helpers can't easily re-use `super::internal` from
/// `persons.rs` (it's a private fn), so we duplicate the one-liner.
fn internal<E: std::fmt::Display>(e: E) -> ApiError {
    ApiError::Internal(anyhow::anyhow!(e.to_string()))
}

/// Validate the contact fields on a draft before it hits the DB.
///
/// - Every contact string is capped at [`CONTACT_FIELD_MAX_LEN`] so a single
///   field can't be abused as a notes dump.
/// - `email` (when non-empty AND not about to be overwritten by the
///   linked-user sync path) must look like an email.
pub fn validate_contact_fields(d: &PersonDraft, email_overridden: bool) -> Result<(), ApiError> {
    let max = u32::try_from(CONTACT_FIELD_MAX_LEN).unwrap_or(u32::MAX);
    let too_long = |path: &str, v: &str| -> Option<ApiError> {
        (v.chars().count() > CONTACT_FIELD_MAX_LEN).then(|| string_too_long(path, max))
    };
    if let Some(e) = too_long("/email", &d.email) {
        return Err(e);
    }
    if let Some(e) = too_long("/phone", &d.phone) {
        return Err(e);
    }
    if let Some(e) = too_long("/street", &d.street) {
        return Err(e);
    }
    if let Some(e) = too_long("/house_number", &d.house_number) {
        return Err(e);
    }
    if let Some(e) = too_long("/zip", &d.zip) {
        return Err(e);
    }
    if let Some(e) = too_long("/city", &d.city) {
        return Err(e);
    }
    if let Some(e) = too_long("/country", &d.country) {
        return Err(e);
    }
    // Only validate email syntax when the caller-provided value will actually
    // hit the DB. When `linked_user_id` is set we'll overwrite with the
    // linked `users.email` value (guaranteed valid by the auth flow) and any
    // body-side garbage is ignored, so validating it would produce confusing
    // 422s for honest clients.
    if !email_overridden && !d.email.is_empty() && !looks_like_email(&d.email) {
        return Err(email_invalid("/email"));
    }
    Ok(())
}

/// Apply the "email is synced from the linked user" rule on a draft.
///
/// When `draft.linked_user_id` is `Some`, look up `users.email` and rewrite
/// `draft.email` to that value, regardless of what the body said. A stale id
/// (the user vanished between request build-time and the DB lookup) maps to
/// [`ApiError::ConflictStale`] — the caller can resync and retry. Returns
/// `true` when the override fired so the validator can skip the email-syntax
/// check (the linked `users.email` is guaranteed valid by the auth flow).
#[allow(clippy::future_not_send, reason = "AppState repos are Arc<dyn _> trait objects")]
pub async fn sync_email_from_linked_user(
    state: &AppState,
    draft: &mut PersonDraft,
) -> Result<bool, ApiError> {
    let Some(uid) = draft.linked_user_id else {
        return Ok(false);
    };
    let user =
        state.users.find_by_id(uid).await.map_err(internal)?.ok_or(ApiError::ConflictStale)?;
    draft.email = user.email;
    Ok(true)
}
