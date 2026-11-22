//! `POST /persons/{id}/claim` — consent-safe self-link.

#![expect(
    clippy::future_not_send,
    reason = "actix handlers hold !Send request types (HttpRequest, Multipart); actix runs each worker on a single thread"
)]
#![expect(
    unreachable_pub,
    reason = "actix route macros turn each handler into a pub service struct"
)]

use actix_web::{HttpRequest, post, web};
use my_fam_tree_domain::{PersonId, Role};
use uuid::Uuid;

use super::support::{map_person_repo_err, to_view};
use super::{PersonView, PersonViewResponseBody};
use crate::auth::{require_role, user_claims_with_family};
use crate::response::ApiResponse;
use crate::services::audit;
use crate::{ApiError, AppState};

// ---------------------------------------------------------------------------
// POST /persons/{id}/claim
// ---------------------------------------------------------------------------

/// Self-claim a person row.
///
/// The signed-in caller links `person.linked_user_id` to themselves. The
/// existing PATCH `/persons/{id}` already accepts an arbitrary
/// `linked_user_id` payload, but that path leaks consent: an admin can
/// silently bind any person row to any user with no notification on the
/// linked user's side. This dedicated endpoint is the consent-safe
/// variant — it only ever links the *caller*, never anyone else, and is
/// the back-end for the "Claim as me" button in `PersonDetail`.
///
/// Gated to admin / owner: regular `user`s onboard via the invite-email
/// round-trip (which doubles as cross-mailbox consent), so they don't need
/// this shortcut. Admins / owners who created a person row for themselves
/// can claim it in one click.
///
/// Rejects (409 [`ApiError::ConflictStale`]) when the person is already
/// linked to anyone (including the caller — a re-claim is a no-op rather
/// than an error worth surfacing to the user), or when the caller is
/// already linked to a different person in this family (the schema
/// enforces uniqueness `(family_id, linked_user_id)`; checking here gives
/// a clean error path instead of the bare
/// [`PersonRepoError::LinkedUserConflict`] mapping).
#[utoipa::path(
    post,
    path = "/api/v1/persons/{id}/claim",
    operation_id = "persons_claim",
    params(("id" = Uuid, Path, description = "Person id")),
    responses(
        (status = 200, description = "Person claimed by caller", body = PersonViewResponseBody),
        (status = 401, description = "No session"),
        (status = 403, description = "Admin or owner required"),
        (status = 404, description = "Not found in this family"),
        (status = 409,
         description = "Already linked, or caller already linked to another person in this family"),
    ),
    security(("cookie_access" = [])),
    tag = "persons",
)]
#[post("/persons/{id}/claim")]
pub async fn claim(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<Uuid>,
) -> Result<ApiResponse<PersonView>, ApiError> {
    let (claims, active) = user_claims_with_family(&req)?;
    require_role(&active, Role::Admin)?;
    let id = path.into_inner();
    let person_id = PersonId::from_uuid(id);

    let existing = state
        .persons
        .find_in_family(active.id, person_id)
        .await
        .map_err(|e| map_person_repo_err(e, Some(id)))?
        .ok_or(ApiError::PersonNotFound { id: Some(id) })?;

    if existing.linked_user_id.is_some() {
        return Err(ApiError::ConflictStale);
    }

    if let Some(already) = state
        .persons
        .find_by_linked_user(active.id, claims.user_id)
        .await
        .map_err(|e| map_person_repo_err(e, None))?
    {
        tracing::info!(
            user_id = %claims.user_id.into_uuid(),
            existing_person_id = %already.id.into_uuid(),
            attempted_person_id = %id,
            "claim rejected: caller already linked to a different person in this family"
        );
        return Err(ApiError::ConflictStale);
    }

    state
        .persons
        .set_linked_user_id(active.id, person_id, Some(claims.user_id))
        .await
        .map_err(|e| map_person_repo_err(e, Some(id)))?;

    audit::record(
        &state.audit,
        active.id,
        claims.user_id,
        "claim",
        "person",
        Some(id),
        serde_json::json!({}),
    )
    .await;

    // Re-fetch so the response reflects the new `linked_user_id`. We
    // can't reuse `existing` (it was the pre-claim snapshot), and
    // `set_linked_user_id` returns `()` so we don't get a row back from
    // the write itself.
    let updated = state
        .persons
        .find_in_family(active.id, person_id)
        .await
        .map_err(|e| map_person_repo_err(e, Some(id)))?
        .ok_or(ApiError::PersonNotFound { id: Some(id) })?;
    Ok(ApiResponse::ok(to_view(updated, &state.object_store).await))
}
