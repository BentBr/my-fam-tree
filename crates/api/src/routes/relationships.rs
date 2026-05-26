//! `GET /api/v1/relationships` — full denormalized tree for the active family.

use actix_web::{HttpRequest, get, web};

use crate::auth::user_claims_with_family;
use crate::response::ApiResponse;
use crate::services::relationships_tree::{TreePayload, build_tree};
use crate::{ApiError, AppState, response_body};

response_body!(pub TreePayloadResponseBody, TreePayload);

#[utoipa::path(
    get,
    path = "/api/v1/relationships",
    responses(
        (status = 200, description = "Tree payload", body = TreePayloadResponseBody),
        (status = 401, description = "No session"),
    ),
    security(("cookie_access" = [])),
    tag = "relationships",
)]
#[allow(clippy::future_not_send)]
#[allow(unreachable_pub)]
#[get("/relationships")]
pub async fn tree(
    state: web::Data<AppState>,
    req: HttpRequest,
) -> Result<ApiResponse<TreePayload>, ApiError> {
    let (claims, active) = user_claims_with_family(&req)?;
    let payload = build_tree(
        &state.persons,
        &state.parent_links,
        &state.partnerships,
        &state.favourites,
        active.id,
        claims.user_id,
    )
    .await
    .map_err(ApiError::Internal)?;
    Ok(ApiResponse::ok(payload))
}
