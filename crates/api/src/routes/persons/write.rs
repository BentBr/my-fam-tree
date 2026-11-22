//! `POST /persons`, `PATCH /persons/{id}` and `DELETE /persons/{id}`.

#![expect(
    clippy::future_not_send,
    reason = "actix handlers hold !Send request types (HttpRequest, Multipart); actix runs each worker on a single thread"
)]
#![expect(
    unreachable_pub,
    reason = "actix route macros turn each handler into a pub service struct"
)]

use actix_web::{HttpRequest, delete, patch, post, web};
use my_fam_tree_domain::{PersonId, Role};
use uuid::Uuid;

use super::support::{
    check_draft, check_link_consent, draft_from_create, map_person_repo_err, merge_update, to_view,
};
use super::{PersonCreateReq, PersonUpdateReq, PersonView, PersonViewResponseBody};
use crate::auth::{require_role, user_claims_with_family};
use crate::response::ApiResponse;
use crate::services::audit;
use crate::validation::value_required;
use crate::{ApiError, AppState};

// ---------------------------------------------------------------------------
// POST /persons
// ---------------------------------------------------------------------------

#[utoipa::path(
    post,
    path = "/api/v1/persons",
    operation_id = "persons_create",
    request_body = PersonCreateReq,
    responses(
        (status = 200, description = "Person created", body = PersonViewResponseBody),
        (status = 401, description = "No session"),
        (status = 422, description = "Validation failed"),
    ),
    security(("cookie_access" = [])),
    tag = "persons",
)]
#[post("/persons")]
pub async fn create(
    state: web::Data<AppState>,
    req: HttpRequest,
    body: web::Json<PersonCreateReq>,
) -> Result<ApiResponse<PersonView>, ApiError> {
    let (claims, active) = user_claims_with_family(&req)?;
    require_role(&active, Role::Admin)?;

    let payload = body.into_inner();
    if payload.given_name.trim().is_empty() {
        return Err(value_required("/given_name"));
    }
    // Consent gate: CREATE may NOT pre-link the new row to anyone other
    // than the caller. See `check_link_consent` doc — a new row has no
    // existing link, so the only allowed Some(_) value is the caller's
    // own user id (admin self-link). Cross-user links go via invite.
    check_link_consent(
        payload.linked_user_id.map(my_fam_tree_domain::UserId::from_uuid),
        None,
        claims.user_id,
    )?;

    let draft = draft_from_create(payload);
    check_draft(&draft)?;
    let person =
        state.persons.create(active.id, draft).await.map_err(|e| map_person_repo_err(e, None))?;
    audit::record(
        &state.audit,
        active.id,
        claims.user_id,
        "create",
        "person",
        Some(person.id.into_uuid()),
        serde_json::json!({}),
    )
    .await;
    Ok(ApiResponse::ok(to_view(person, &state.object_store).await))
}

// ---------------------------------------------------------------------------
// PATCH /persons/{id}
// ---------------------------------------------------------------------------

#[utoipa::path(
    patch,
    path = "/api/v1/persons/{id}",
    operation_id = "persons_update",
    request_body = PersonUpdateReq,
    params(("id" = Uuid, Path, description = "Person id")),
    responses(
        (status = 200, description = "Person updated", body = PersonViewResponseBody),
        (status = 401, description = "No session"),
        (status = 403, description = "Not allowed to edit this person"),
        (status = 404, description = "Not found in this family"),
        (status = 422, description = "Validation failed"),
    ),
    security(("cookie_access" = [])),
    tag = "persons",
)]
#[patch("/persons/{id}")]
pub async fn update(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<Uuid>,
    body: web::Json<PersonUpdateReq>,
) -> Result<ApiResponse<PersonView>, ApiError> {
    let (claims, active) = user_claims_with_family(&req)?;
    let id = path.into_inner();
    let person_id = PersonId::from_uuid(id);

    let existing = state
        .persons
        .find_in_family(active.id, person_id)
        .await
        .map_err(|e| map_person_repo_err(e, Some(id)))?
        .ok_or(ApiError::PersonNotFound { id: Some(id) })?;

    // Regular users may only edit the person row that maps to themselves.
    if active.role == Role::User && existing.linked_user_id != Some(claims.user_id) {
        return Err(ApiError::PersonNotEditable);
    }

    let payload = body.into_inner();
    // Reject a fully-empty PATCH so the caller knows nothing happened.
    let any_change = payload.given_name.is_some()
        || payload.family_name.is_some()
        || payload.name_at_birth.is_some()
        || payload.nickname.is_some()
        || payload.gender.is_some()
        || payload.birth_date.is_some()
        || payload.birth_place.is_some()
        || payload.death_date.is_some()
        || payload.notes.is_some()
        || payload.linked_user_id.is_some();
    if !any_change {
        return Err(value_required("/"));
    }
    if matches!(payload.given_name.as_deref(), Some(s) if s.trim().is_empty()) {
        return Err(value_required("/given_name"));
    }
    // Consent gate: PATCH may NOT change `linked_user_id` to anyone
    // other than the caller. The defensive-echo case (FE re-sends
    // the existing value as part of a wider edit) is allowed by
    // `check_link_consent`; the genuine change case routes through
    // POST /persons/{id}/claim (self) or the invite-email flow
    // (cross-user). See the helper's doc for the consent rationale.
    check_link_consent(
        payload.linked_user_id.map(my_fam_tree_domain::UserId::from_uuid),
        existing.linked_user_id,
        claims.user_id,
    )?;

    let draft = merge_update(&existing, payload);
    check_draft(&draft)?;
    let person = state
        .persons
        .update(active.id, person_id, draft)
        .await
        .map_err(|e| map_person_repo_err(e, Some(id)))?;
    audit::record(
        &state.audit,
        active.id,
        claims.user_id,
        "update",
        "person",
        Some(id),
        serde_json::json!({}),
    )
    .await;
    Ok(ApiResponse::ok(to_view(person, &state.object_store).await))
}

// ---------------------------------------------------------------------------
// DELETE /persons/{id}
// ---------------------------------------------------------------------------

#[utoipa::path(
    delete,
    path = "/api/v1/persons/{id}",
    operation_id = "persons_delete",
    params(("id" = Uuid, Path, description = "Person id")),
    responses(
        (status = 200, description = "Person deleted", body = crate::response::NullResponseBody),
        (status = 401, description = "No session"),
        (status = 403, description = "Admin or owner required"),
        (status = 404, description = "Not found in this family"),
    ),
    security(("cookie_access" = [])),
    tag = "persons",
)]
#[delete("/persons/{id}")]
pub async fn delete(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<Uuid>,
) -> Result<ApiResponse<serde_json::Value>, ApiError> {
    let (claims, active) = user_claims_with_family(&req)?;
    require_role(&active, Role::Admin)?;
    let id = path.into_inner();
    state
        .persons
        .delete(active.id, PersonId::from_uuid(id))
        .await
        .map_err(|e| map_person_repo_err(e, Some(id)))?;
    audit::record(
        &state.audit,
        active.id,
        claims.user_id,
        "delete",
        "person",
        Some(id),
        serde_json::json!({}),
    )
    .await;
    // Spec § 5: DELETE returns `{ "data": null }`, not a status string.
    Ok(ApiResponse::ok(serde_json::Value::Null))
}
