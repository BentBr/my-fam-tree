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
use crate::validation::value_required;
use crate::{ApiError, AppState};

#[derive(Debug, Deserialize, ToSchema)]
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
        (status = 409, description = "Relationship would create a cycle"),
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
    let (_claims, active) = user_claims_with_family(&req)?;
    require_role(&active, Role::Admin)?;

    let payload = body.into_inner();
    let kind = parse_kind(&payload.kind).ok_or_else(|| value_required("/kind"))?;
    let child = PersonId::from_uuid(payload.child_id);
    let parent = PersonId::from_uuid(payload.parent_id);

    if child == parent {
        return Err(ApiError::RelationshipCycle);
    }

    let edges =
        state.parent_links.all_edges_in_family(active.id).await.map_err(internal_link_err)?;
    if would_create_cycle(&edges, child, parent) {
        return Err(ApiError::RelationshipCycle);
    }

    state
        .parent_links
        .insert(active.id, child, parent, kind, &payload.note)
        .await
        .map_err(internal_link_err)?;
    Ok(ApiResponse::ok(serde_json::Value::Null))
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
    let (_claims, active) = user_claims_with_family(&req)?;
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
    Ok(ApiResponse::ok(serde_json::Value::Null))
}

fn internal_link_err(e: ParentLinkRepoError) -> ApiError {
    match e {
        ParentLinkRepoError::Cycle | ParentLinkRepoError::SelfParent => ApiError::RelationshipCycle,
        other => internal(other),
    }
}
