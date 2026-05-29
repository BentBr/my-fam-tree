//! `GET /api/v1/families/{family_id}/audit` — admin+owner only.
//!
//! Returns the `audit_log` rows for the active family, filtered + paged.
//! `entity_person_id` is resolved inside the persistence query
//! (see [`my_fam_tree_persistence::audit_log::PgAuditLogRepo::list_filtered`])
//! so the FE can render a one-click `/tree?center=<personId>` deep
//! link without a second round-trip.
//!
//! Role gate: `require_role(Role::Admin)` early-returns 403 before any
//! DB work. The path's `family_id` is double-checked against the
//! `X-Family-Id`-derived `ActiveFamily` to keep the URL and the
//! resolved family in sync — a mismatch is a 401, because the only way
//! to land here with a different `family_id` is a forged URL.

use actix_web::{HttpRequest, get, web};
use chrono::{DateTime, Utc};
use my_fam_tree_domain::{AuditFilter, FamilyId, Role, UserId};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::auth::{require_db_role, user_claims_with_family};
use crate::response::ApiResponse;
use crate::{ApiError, AppState, response_body};

#[derive(Debug, Deserialize, IntoParams)]
pub struct AuditQuery {
    /// 1-based page index. Defaults to 1; values below 1 are clamped.
    pub page: Option<u32>,
    /// One of 50 / 100 / 200 / 500. Anything else falls back to 50.
    pub page_size: Option<u32>,
    /// Inclusive lower bound on `created_at`.
    pub from: Option<DateTime<Utc>>,
    /// Inclusive upper bound on `created_at`.
    pub to: Option<DateTime<Utc>>,
    /// Short verb slug — e.g. `create`, `update`, `delete`, `invite`.
    pub action: Option<String>,
    /// Noun slug — e.g. `person`, `contact`, `membership`, `invite`.
    pub entity_kind: Option<String>,
    /// Restrict to rows written by this user.
    pub actor_user_id: Option<Uuid>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AuditRowDto {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    pub action: String,
    pub entity_kind: String,
    pub entity_id: Option<Uuid>,
    pub metadata: serde_json::Value,
    pub actor_user_id: Option<Uuid>,
    pub actor_display_name: Option<String>,
    pub actor_email: Option<String>,
    pub entity_person_id: Option<Uuid>,
    pub entity_person_name: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AuditPage {
    pub data: Vec<AuditRowDto>,
    pub page: u32,
    pub page_size: u32,
    pub total: i64,
}

response_body!(pub AuditPageResponseBody, AuditPage);

fn internal<E: std::fmt::Display>(e: E) -> ApiError {
    ApiError::Internal(anyhow::anyhow!(e.to_string()))
}

const fn clamp_page_size(requested: u32) -> u32 {
    match requested {
        50 | 100 | 200 | 500 => requested,
        _ => 50,
    }
}

#[utoipa::path(
    get,
    path = "/api/v1/families/{family_id}/audit",
    operation_id = "audit_list",
    params(
        ("family_id" = Uuid, Path, description = "Family id (must match the active X-Family-Id)"),
        AuditQuery,
    ),
    responses(
        (status = 200, description = "Page of audit rows", body = AuditPageResponseBody),
        (status = 401, description = "Path family_id does not match active family"),
        (status = 403, description = "Insufficient role (admin / owner required)"),
    ),
    security(("cookie_access" = [])),
    tag = "audit",
)]
#[allow(clippy::future_not_send)]
#[allow(unreachable_pub)]
#[get("/families/{family_id}/audit")]
pub async fn list_audit(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<Uuid>,
    q: web::Query<AuditQuery>,
) -> Result<ApiResponse<AuditPage>, ApiError> {
    let (claims, active) = user_claims_with_family(&req)?;
    require_db_role(&state, claims.user_id, active.id, Role::Admin).await?;

    let family_id = FamilyId::from_uuid(path.into_inner());
    if active.id != family_id {
        return Err(ApiError::Unauthenticated);
    }

    let page = q.page.unwrap_or(1).max(1);
    let page_size = clamp_page_size(q.page_size.unwrap_or(50));

    let filter = AuditFilter {
        family_id,
        from: q.from,
        to: q.to,
        action: q.action.clone(),
        entity_kind: q.entity_kind.clone(),
        actor_user_id: q.actor_user_id.map(UserId::from_uuid),
        page,
        page_size,
    };

    let (rows, meta) = state.audit.list_filtered(filter).await.map_err(internal)?;

    let dtos: Vec<AuditRowDto> = rows
        .into_iter()
        .map(|r| AuditRowDto {
            id: r.id,
            created_at: r.created_at,
            action: r.action,
            entity_kind: r.entity_kind,
            entity_id: r.entity_id,
            metadata: r.metadata,
            actor_user_id: r.actor_user_id.map(UserId::into_uuid),
            actor_display_name: r.actor_display_name,
            actor_email: r.actor_email,
            entity_person_id: r.entity_person_id,
            entity_person_name: r.entity_person_name,
        })
        .collect();

    Ok(ApiResponse::ok(AuditPage { data: dtos, page, page_size, total: meta.total }))
}
