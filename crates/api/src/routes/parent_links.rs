//! `/parent-links` — directed parent→child edges.
//!
//! Two endpoints, both admin/owner only:
//! - `POST /parent-links` — add a `(child, parent, kind)` edge. Rejects
//!   self-parent and cycles. The cycle check reads the family's current edge
//!   set in-memory; under concurrent inserts a race could slip a cycle in.
//!   The plan defers a SERIALIZABLE-tx hardening to Phase 5.
//! - `DELETE /parent-links/{child}/{parent}` — remove the edge.

use actix_web::{HttpRequest, delete, post, web};
use my_family_domain::{ParentKind, ParentLinkRepoError, PersonId, Role, would_create_cycle};
use serde::Deserialize;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::auth::{require_role, user_claims_with_family};
use crate::response::{ApiResponse, NullResponseBody};
use crate::services::audit;
use crate::validation::relationships::check_parent_link;
use crate::validation::value_required;
use crate::{ApiError, AppState};

#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ParentLinkReq {
    pub child_id: Uuid,
    pub parent_id: Uuid,
    /// One of `biological`, `legal`, `adoptive`, `step`, `social`.
    pub kind: String,
    #[serde(default)]
    pub note: String,
}

fn internal<E: std::fmt::Display>(e: E) -> ApiError {
    ApiError::Internal(anyhow::anyhow!(e.to_string()))
}

fn parse_kind(raw: &str) -> Option<ParentKind> {
    match raw {
        "biological" => Some(ParentKind::Biological),
        "legal" => Some(ParentKind::Legal),
        "adoptive" => Some(ParentKind::Adoptive),
        "step" => Some(ParentKind::Step),
        "social" => Some(ParentKind::Social),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// POST /parent-links
// ---------------------------------------------------------------------------

#[utoipa::path(
    post,
    path = "/api/v1/parent-links",
    operation_id = "parent_links_create",
    request_body = ParentLinkReq,
    responses(
        (status = 200, description = "Edge inserted", body = NullResponseBody),
        (status = 401, description = "No session"),
        (status = 403, description = "Admin or owner required"),
        (status = 409, description = "Cycle or duplicate edge"),
        (status = 422, description = "Validation failed"),
    ),
    security(("cookie_access" = [])),
    tag = "relationships",
)]
#[allow(clippy::future_not_send)]
#[allow(unreachable_pub)]
#[post("/parent-links")]
pub async fn create(
    state: web::Data<AppState>,
    req: HttpRequest,
    body: web::Json<ParentLinkReq>,
) -> Result<ApiResponse<serde_json::Value>, ApiError> {
    let (claims, active) = user_claims_with_family(&req)?;
    require_role(&active, Role::Admin)?;

    let payload = body.into_inner();
    let kind = parse_kind(&payload.kind).ok_or_else(|| value_required("/kind"))?;
    let child = PersonId::from_uuid(payload.child_id);
    let parent = PersonId::from_uuid(payload.parent_id);

    if child == parent {
        return Err(ApiError::RelationshipCycle);
    }

    // Cross-family IDOR guard (security audit HIGH). `parent_links` has no
    // `family_id` column — without this check an F1 admin could POST a row
    // referencing two F2 persons; the join in `list_for_family` would then
    // surface the rogue edge inside F2's tree. Resolve both endpoints
    // through `find_in_family(active.id, _)` so the request is rejected
    // at the edge with a clear `person.not_found`.
    if state
        .persons
        .find_in_family(active.id, child)
        .await
        .map_err(|e| internal(format!("persons.find_in_family: {e}")))?
        .is_none()
    {
        return Err(ApiError::PersonNotFound { id: Some(child.into_uuid()) });
    }
    if state
        .persons
        .find_in_family(active.id, parent)
        .await
        .map_err(|e| internal(format!("persons.find_in_family: {e}")))?
        .is_none()
    {
        return Err(ApiError::PersonNotFound { id: Some(parent.into_uuid()) });
    }

    let edges =
        state.parent_links.all_edges_in_family(active.id).await.map_err(internal_link_err)?;
    if would_create_cycle(&edges, child, parent) {
        return Err(ApiError::RelationshipCycle);
    }

    // Cross-aggregate validation: parent age, deceased-before-birth,
    // bio-parent cap. Pulls a fresh snapshot of the family graph so the
    // rules see the same view the cycle check just read.
    let persons = state
        .persons
        .list_for_family(active.id, None, 100)
        .await
        .map_err(|e| internal(format!("persons.list_for_family: {e}")))?;
    let links = state.parent_links.list_for_family(active.id).await.map_err(internal_link_err)?;
    // Fast-path: surface a duplicate edge before running the cross-aggregate
    // validations or hitting the DB. The repo's INSERT is the race-safe
    // backstop; this just spares the round-trip on the common case where
    // the FE clicked "add" twice or replayed an idempotent request.
    if links.iter().any(|l| l.child_id == child && l.parent_id == parent) {
        return Err(ApiError::ParentLinkDuplicate);
    }
    let warnings = check_parent_link(child, parent, kind, &persons, &links)?;

    state
        .parent_links
        .insert(active.id, child, parent, kind, &payload.note)
        .await
        .map_err(internal_link_err)?;
    audit::record(
        &state.audit,
        active.id,
        claims.user_id,
        "create",
        "parent_link",
        None,
        serde_json::json!({
            "child_id": child.into_uuid(),
            "parent_id": parent.into_uuid(),
            "kind": payload.kind,
        }),
    )
    .await;
    Ok(ApiResponse::ok(serde_json::Value::Null).with_warnings(warnings))
}

// ---------------------------------------------------------------------------
// DELETE /parent-links/{child}/{parent}
// ---------------------------------------------------------------------------

#[utoipa::path(
    delete,
    path = "/api/v1/parent-links/{child}/{parent}",
    operation_id = "parent_links_delete",
    params(
        ("child" = Uuid, Path, description = "Child person id"),
        ("parent" = Uuid, Path, description = "Parent person id"),
    ),
    responses(
        (status = 200, description = "Edge deleted", body = NullResponseBody),
        (status = 401, description = "No session"),
        (status = 403, description = "Admin or owner required"),
        (status = 404, description = "Edge not found"),
    ),
    security(("cookie_access" = [])),
    tag = "relationships",
)]
#[allow(clippy::future_not_send)]
#[allow(unreachable_pub)]
#[delete("/parent-links/{child}/{parent}")]
pub async fn delete(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<(Uuid, Uuid)>,
) -> Result<ApiResponse<serde_json::Value>, ApiError> {
    let (claims, active) = user_claims_with_family(&req)?;
    require_role(&active, Role::Admin)?;
    let (child, parent) = path.into_inner();
    state
        .parent_links
        .delete(active.id, PersonId::from_uuid(child), PersonId::from_uuid(parent))
        .await
        .map_err(|e| match e {
            ParentLinkRepoError::NotFound => ApiError::PersonNotFound { id: Some(child) },
            other => internal(other),
        })?;
    audit::record(
        &state.audit,
        active.id,
        claims.user_id,
        "delete",
        "parent_link",
        None,
        serde_json::json!({
            "child_id": child,
            "parent_id": parent,
        }),
    )
    .await;
    Ok(ApiResponse::ok(serde_json::Value::Null))
}

fn internal_link_err(e: ParentLinkRepoError) -> ApiError {
    match e {
        ParentLinkRepoError::Cycle | ParentLinkRepoError::SelfParent => ApiError::RelationshipCycle,
        ParentLinkRepoError::Duplicate => ApiError::ParentLinkDuplicate,
        other => internal(other),
    }
}
