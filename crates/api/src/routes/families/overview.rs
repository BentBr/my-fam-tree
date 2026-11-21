//! `GET /admin/family/overview` — aggregated stats for the admin page.

use actix_web::{HttpRequest, get, web};
use my_fam_tree_domain::Role;

use super::{FamilyOverview, FamilyOverviewResponseBody, LatestPersonView, internal};
use crate::{ApiError, ApiResponse, AppState};

// ---------------------------------------------------------------------------
// GET /admin/family/overview
// ---------------------------------------------------------------------------

#[utoipa::path(
    get,
    path = "/api/v1/admin/family/overview",
    responses(
        (status = 200, description = "Active family overview (admin+owner only)", body = FamilyOverviewResponseBody),
        (status = 401, description = "No session"),
        (status = 403, description = "Insufficient role"),
        (status = 422, description = "Missing X-Family-Id header"),
    ),
    security(("cookie_access" = [])),
    tag = "families",
)]
#[allow(clippy::future_not_send)]
#[allow(unreachable_pub)]
#[get("/admin/family/overview")]
pub async fn admin_overview(
    state: web::Data<AppState>,
    req: HttpRequest,
) -> Result<ApiResponse<FamilyOverview>, ApiError> {
    // Active family resolution: takes X-Family-Id from the request and
    // matches it against the caller's JWT memberships. Then the DB role
    // check makes sure they're admin / owner — the route guard on the
    // FE also enforces this, but the server is the source of truth.
    let (claims, active) = crate::auth::user_claims_with_family(&req)?;
    crate::auth::require_db_role(&state, claims.user_id, active.id, Role::Admin).await?;

    // The membership check above already proves the family exists in the
    // caller's claims; a `None` here would be an internal consistency
    // bug (membership row pointing at a deleted family), not a
    // user-facing 404.
    let family =
        state.families.find_by_id(active.id).await.map_err(internal)?.ok_or_else(|| {
            ApiError::Internal(anyhow::anyhow!("family vanished for active membership"))
        })?;
    // Three aggregations against the same family. Parallel awaits keep
    // the cumulative latency tight; the page is one of those "load
    // everything at once" surfaces, not a hot path.
    let (members, persons, latest) = tokio::join!(
        state.memberships.count_in_family(active.id),
        state.persons.count_in_family(active.id),
        state.persons.list_latest_in_family(active.id, 3),
    );
    let member_count = members.map_err(internal)?;
    let person_count = persons.map_err(internal)?;
    let latest_persons = latest
        .map_err(internal)?
        .into_iter()
        .map(|p| LatestPersonView {
            id: p.id.into_uuid(),
            given_name: p.given_name,
            family_name: p.family_name,
            created_at: p.created_at,
        })
        .collect();
    Ok(ApiResponse::ok(FamilyOverview {
        id: active.id.into_uuid(),
        name: family.name,
        role: active.role,
        member_count,
        person_count,
        latest_persons,
    }))
}
