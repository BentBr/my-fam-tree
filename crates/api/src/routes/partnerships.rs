//! `/partnerships` — pair-canonicalized partnership rows.
//!
//! Three endpoints, all admin/owner only:
//! - `POST /partnerships` — create. The DB unique index keys on
//!   `(partner_a_id, partner_b_id, kind) WHERE ended_on IS NULL`, so duplicate
//!   currently-open partnerships surface as `partnership.duplicate`.
//! - `PATCH /partnerships/{id}` — update kind / dates / `end_reason` / note.
//! - `DELETE /partnerships/{id}` — remove.

use actix_web::{HttpRequest, delete, patch, post, web};
use chrono::NaiveDate;
use my_fam_tree_domain::{
    Partnership, PartnershipDraft, PartnershipEndReason, PartnershipKind, PartnershipRepoError,
    PersonId, Role,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::auth::{require_role, user_claims_with_family};
use crate::response::{ApiResponse, NullResponseBody};
use crate::services::audit;
use crate::validation::relationships::check_partnership;
use crate::validation::value_required;
use crate::{ApiError, AppState, response_body};

#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct PartnershipCreateReq {
    pub partner_a_id: Uuid,
    pub partner_b_id: Uuid,
    /// One of `marriage`, `civil_union`, `partnership`.
    pub kind: String,
    pub started_on: Option<NaiveDate>,
    pub ended_on: Option<NaiveDate>,
    /// One of `divorce`, `separation`, `death`.
    pub end_reason: Option<String>,
    #[serde(default)]
    pub note: String,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct PartnershipUpdateReq {
    pub kind: Option<String>,
    pub started_on: Option<NaiveDate>,
    pub ended_on: Option<NaiveDate>,
    pub end_reason: Option<String>,
    pub note: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PartnershipView {
    pub id: Uuid,
    pub family_id: Uuid,
    pub partner_a_id: Uuid,
    pub partner_b_id: Uuid,
    pub kind: String,
    pub started_on: Option<NaiveDate>,
    pub ended_on: Option<NaiveDate>,
    pub end_reason: Option<String>,
    pub note: String,
}

response_body!(pub PartnershipViewResponseBody, PartnershipView);

fn internal<E: std::fmt::Display>(e: E) -> ApiError {
    ApiError::Internal(anyhow::anyhow!(e.to_string()))
}

fn parse_kind(raw: &str) -> Option<PartnershipKind> {
    match raw {
        "marriage" => Some(PartnershipKind::Marriage),
        "civil_union" => Some(PartnershipKind::CivilUnion),
        "partnership" => Some(PartnershipKind::Partnership),
        _ => None,
    }
}

fn parse_end_reason(raw: &str) -> Option<PartnershipEndReason> {
    match raw {
        "divorce" => Some(PartnershipEndReason::Divorce),
        "separation" => Some(PartnershipEndReason::Separation),
        "death" => Some(PartnershipEndReason::Death),
        _ => None,
    }
}

const fn kind_to_str(k: PartnershipKind) -> &'static str {
    k.as_db()
}

const fn end_to_str(e: PartnershipEndReason) -> &'static str {
    e.as_db()
}

fn to_view(p: Partnership) -> PartnershipView {
    PartnershipView {
        id: p.id,
        family_id: p.family_id.into_uuid(),
        partner_a_id: p.partner_a_id.into_uuid(),
        partner_b_id: p.partner_b_id.into_uuid(),
        kind: kind_to_str(p.kind).to_string(),
        started_on: p.started_on,
        ended_on: p.ended_on,
        end_reason: p.end_reason.map(|r| end_to_str(r).to_string()),
        note: p.note,
    }
}

fn map_repo_err(e: PartnershipRepoError, id: Option<Uuid>) -> ApiError {
    match e {
        PartnershipRepoError::Duplicate => ApiError::PartnershipDuplicate,
        PartnershipRepoError::NotFound => id.map_or_else(
            || internal(PartnershipRepoError::NotFound),
            |i| ApiError::PersonNotFound { id: Some(i) },
        ),
        other => internal(other),
    }
}

// ---------------------------------------------------------------------------
// POST /partnerships
// ---------------------------------------------------------------------------

#[utoipa::path(
    post,
    path = "/api/v1/partnerships",
    operation_id = "partnerships_create",
    request_body = PartnershipCreateReq,
    responses(
        (status = 200, description = "Partnership created", body = PartnershipViewResponseBody),
        (status = 401, description = "No session"),
        (status = 403, description = "Admin or owner required"),
        (status = 409, description = "Duplicate open partnership"),
        (status = 422, description = "Validation failed"),
    ),
    security(("cookie_access" = [])),
    tag = "relationships",
)]
#[allow(clippy::future_not_send)]
#[allow(unreachable_pub)]
#[post("/partnerships")]
pub async fn create(
    state: web::Data<AppState>,
    req: HttpRequest,
    body: web::Json<PartnershipCreateReq>,
) -> Result<ApiResponse<PartnershipView>, ApiError> {
    let (claims, active) = user_claims_with_family(&req)?;
    require_role(&active, Role::Admin)?;

    let payload = body.into_inner();
    let kind = parse_kind(&payload.kind).ok_or_else(|| value_required("/kind"))?;
    let end_reason = match payload.end_reason.as_deref() {
        None => None,
        Some(raw) => Some(parse_end_reason(raw).ok_or_else(|| value_required("/end_reason"))?),
    };
    let a = PersonId::from_uuid(payload.partner_a_id);
    let b = PersonId::from_uuid(payload.partner_b_id);

    // Cross-family IDOR guard (security audit MEDIUM). Although the
    // partnerships row carries family_id (so list_for_family won't surface
    // a rogue edge), an F1 admin could still pollute F1's own list with a
    // partnership between two F2 persons, and ON DELETE CASCADE would wipe
    // the row when F2 deletes one of those persons. Block at the edge.
    if state
        .persons
        .find_in_family(active.id, a)
        .await
        .map_err(|e| internal(format!("persons.find_in_family: {e}")))?
        .is_none()
    {
        return Err(ApiError::PersonNotFound { id: Some(a.into_uuid()) });
    }
    if state
        .persons
        .find_in_family(active.id, b)
        .await
        .map_err(|e| internal(format!("persons.find_in_family: {e}")))?
        .is_none()
    {
        return Err(ApiError::PersonNotFound { id: Some(b.into_uuid()) });
    }

    // Cross-aggregate validation: partnership-before-birth, sibling
    // warning, death cross-check. Pulls the family graph (same scope and
    // bounds as the relationships-tree service uses elsewhere).
    let persons = state
        .persons
        .list_for_family(active.id, None, 100)
        .await
        .map_err(|e| internal(format!("persons.list_for_family: {e}")))?;
    let parent_links = state
        .parent_links
        .list_for_family(active.id)
        .await
        .map_err(|e| internal(format!("parent_links.list_for_family: {e}")))?;
    let partnerships_now =
        state.partnerships.list_for_family(active.id).await.map_err(|e| map_repo_err(e, None))?;
    let warnings = check_partnership(
        a,
        b,
        payload.started_on,
        payload.ended_on,
        end_reason,
        &persons,
        &parent_links,
        &partnerships_now,
    )?;

    let draft = PartnershipDraft {
        kind,
        started_on: payload.started_on,
        ended_on: payload.ended_on,
        end_reason,
        note: payload.note,
    };
    let partnership = state
        .partnerships
        .create(active.id, a, b, draft)
        .await
        .map_err(|e| map_repo_err(e, None))?;
    audit::record(
        &state.audit,
        active.id,
        claims.user_id,
        "create",
        "partnership",
        Some(partnership.id),
        serde_json::json!({
            "partner_a_id": a.into_uuid(),
            "partner_b_id": b.into_uuid(),
            "kind": partnership.kind.as_db(),
        }),
    )
    .await;
    Ok(ApiResponse::ok(to_view(partnership)).with_warnings(warnings))
}

// ---------------------------------------------------------------------------
// PATCH /partnerships/{id}
// ---------------------------------------------------------------------------

#[utoipa::path(
    patch,
    path = "/api/v1/partnerships/{id}",
    operation_id = "partnerships_update",
    request_body = PartnershipUpdateReq,
    params(("id" = Uuid, Path, description = "Partnership id")),
    responses(
        (status = 200, description = "Partnership updated", body = PartnershipViewResponseBody),
        (status = 401, description = "No session"),
        (status = 403, description = "Admin or owner required"),
        (status = 404, description = "Partnership not found"),
        (status = 422, description = "Validation failed"),
    ),
    security(("cookie_access" = [])),
    tag = "relationships",
)]
#[allow(clippy::future_not_send)]
#[allow(unreachable_pub)]
#[patch("/partnerships/{id}")]
pub async fn update(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<Uuid>,
    body: web::Json<PartnershipUpdateReq>,
) -> Result<ApiResponse<PartnershipView>, ApiError> {
    let (claims, active) = user_claims_with_family(&req)?;
    require_role(&active, Role::Admin)?;
    let id = path.into_inner();

    // Fetch the current row to merge in PATCH semantics; the repo only exposes
    // a full-replace `update`, so we read first, merge, then write back.
    let existing = state
        .partnerships
        .list_for_family(active.id)
        .await
        .map_err(|e| map_repo_err(e, Some(id)))?
        .into_iter()
        .find(|p| p.id == id)
        .ok_or(ApiError::PersonNotFound { id: Some(id) })?;

    let payload = body.into_inner();
    let any_change = payload.kind.is_some()
        || payload.started_on.is_some()
        || payload.ended_on.is_some()
        || payload.end_reason.is_some()
        || payload.note.is_some();
    if !any_change {
        return Err(value_required("/"));
    }

    let kind = match payload.kind.as_deref() {
        None => existing.kind,
        Some(raw) => parse_kind(raw).ok_or_else(|| value_required("/kind"))?,
    };
    let end_reason = match payload.end_reason.as_deref() {
        None => existing.end_reason,
        Some("") => None,
        Some(raw) => Some(parse_end_reason(raw).ok_or_else(|| value_required("/end_reason"))?),
    };

    let draft = PartnershipDraft {
        kind,
        started_on: payload.started_on.or(existing.started_on),
        ended_on: payload.ended_on.or(existing.ended_on),
        end_reason,
        note: payload.note.unwrap_or(existing.note),
    };
    let updated = state
        .partnerships
        .update(active.id, id, draft)
        .await
        .map_err(|e| map_repo_err(e, Some(id)))?;
    audit::record(
        &state.audit,
        active.id,
        claims.user_id,
        "update",
        "partnership",
        Some(id),
        serde_json::json!({}),
    )
    .await;
    Ok(ApiResponse::ok(to_view(updated)))
}

// ---------------------------------------------------------------------------
// DELETE /partnerships/{id}
// ---------------------------------------------------------------------------

#[utoipa::path(
    delete,
    path = "/api/v1/partnerships/{id}",
    operation_id = "partnerships_delete",
    params(("id" = Uuid, Path, description = "Partnership id")),
    responses(
        (status = 200, description = "Partnership deleted", body = NullResponseBody),
        (status = 401, description = "No session"),
        (status = 403, description = "Admin or owner required"),
        (status = 404, description = "Partnership not found"),
    ),
    security(("cookie_access" = [])),
    tag = "relationships",
)]
#[allow(clippy::future_not_send)]
#[allow(unreachable_pub)]
#[delete("/partnerships/{id}")]
pub async fn delete(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<Uuid>,
) -> Result<ApiResponse<serde_json::Value>, ApiError> {
    let (claims, active) = user_claims_with_family(&req)?;
    require_role(&active, Role::Admin)?;
    let id = path.into_inner();
    state.partnerships.delete(active.id, id).await.map_err(|e| map_repo_err(e, Some(id)))?;
    audit::record(
        &state.audit,
        active.id,
        claims.user_id,
        "delete",
        "partnership",
        Some(id),
        serde_json::json!({}),
    )
    .await;
    Ok(ApiResponse::ok(serde_json::Value::Null))
}
