//! `GET /api/v1/upcoming` — projected upcoming dates (birthdays + anniversaries).
//!
//! The handler delegates the projection / sort / limit to
//! [`crate::services::upcoming::build_upcoming`] and only wraps the result
//! in `ApiResponse`. Today's date is read from `chrono::Utc::now()` —
//! the inevitable "midnight rollover" off-by-one across timezones is
//! acceptable for an MVP list view; persons whose birthday is "today
//! in their timezone but not yet in UTC" will simply appear one day
//! earlier than they otherwise might.

use actix_web::{HttpRequest, get, web};
use chrono::Utc;
use serde::Deserialize;
use utoipa::ToSchema;

use crate::auth::user_claims_with_family;
use crate::response::ApiResponse;
use crate::services::upcoming::{
    DEFAULT_LIMIT, MAX_LIMIT, UpcomingEvent, UpcomingFilter, build_upcoming,
};
use crate::{ApiError, AppState, response_body};

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpcomingQuery {
    /// One of `all` (default), `birthday`, `anniversary`. Unknown
    /// values fall through to `all` — the FE never sends arbitrary
    /// strings, but a hand-crafted URL stays gracefully degraded.
    pub filter: Option<String>,
    /// 1..=200, default 20. Clamped to the cap so callers can't
    /// exhaust memory by passing `limit=999999`.
    pub limit: Option<u32>,
}

response_body!(pub UpcomingResponseBody, Vec<UpcomingEvent>);

#[utoipa::path(
    get,
    path = "/api/v1/upcoming",
    operation_id = "upcoming_list",
    params(
        ("filter" = Option<String>, Query, description = "all | birthday | anniversary"),
        ("limit" = Option<u32>, Query, description = "Cap on rows, 1..=200, default 20"),
    ),
    responses(
        (status = 200, description = "Upcoming events", body = UpcomingResponseBody),
        (status = 401, description = "No session"),
    ),
    security(("cookie_access" = [])),
    tag = "upcoming",
)]
#[allow(clippy::future_not_send)]
#[allow(unreachable_pub)]
#[get("/upcoming")]
pub async fn list(
    state: web::Data<AppState>,
    req: HttpRequest,
    query: web::Query<UpcomingQuery>,
) -> Result<ApiResponse<Vec<UpcomingEvent>>, ApiError> {
    let (_claims, active) = user_claims_with_family(&req)?;
    let filter = UpcomingFilter::parse(query.filter.as_deref());
    let limit = query.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);
    let today = Utc::now().date_naive();
    let events =
        build_upcoming(&state.persons, &state.partnerships, active.id, today, filter, limit)
            .await
            .map_err(ApiError::Internal)?;
    Ok(ApiResponse::ok(events))
}
