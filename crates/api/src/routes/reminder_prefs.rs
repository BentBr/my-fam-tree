//! `/reminder-preferences` — the caller's per-user reminder settings.
//!
//! - `GET /reminder-preferences` — current settings, or built-in defaults when
//!   the user has never saved any (emails OFF — opt-in).
//! - `PUT /reminder-preferences` — upsert the single settings row. `lead_days`
//!   must be 0..=21 (0 = on the day); out of range → 422.
//!
//! Both endpoints live under `AuthMiddleware::required`. The worker (Phase 4b)
//! reads these rows to decide who gets a daily digest and when.

use actix_web::{HttpRequest, get, put, web};
use my_family_domain::ReminderPreferences;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::error::FieldViolation;
use crate::{ApiError, ApiResponse, AppState, response_body};

const LEAD_DAYS_MIN: i32 = 0;
const LEAD_DAYS_MAX: i32 = 21;

/// Wire shape for the caller's reminder settings (request + response).
#[allow(
    clippy::struct_excessive_bools,
    reason = "independent user-facing toggles mirroring reminder_preferences, not a state machine"
)]
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ReminderPrefsView {
    pub emails_enabled: bool,
    pub remind_birthdays: bool,
    pub remind_anniversaries: bool,
    pub favourites_only: bool,
    pub lead_days: i32,
}

impl From<ReminderPreferences> for ReminderPrefsView {
    fn from(p: ReminderPreferences) -> Self {
        Self {
            emails_enabled: p.emails_enabled,
            remind_birthdays: p.remind_birthdays,
            remind_anniversaries: p.remind_anniversaries,
            favourites_only: p.favourites_only,
            lead_days: p.lead_days,
        }
    }
}

response_body!(pub ReminderPrefsResponseBody, ReminderPrefsView);

fn internal<E: std::fmt::Display>(e: E) -> ApiError {
    ApiError::Internal(anyhow::anyhow!(e.to_string()))
}

#[utoipa::path(
    get,
    path = "/api/v1/reminder-preferences",
    operation_id = "reminder_prefs_get",
    responses(
        (status = 200, description = "Caller's reminder settings", body = ReminderPrefsResponseBody),
        (status = 401, description = "No session"),
    ),
    security(("cookie_access" = [])),
    tag = "reminders",
)]
#[allow(clippy::future_not_send)]
#[allow(unreachable_pub)]
#[get("/reminder-preferences")]
pub async fn get_prefs(
    state: web::Data<AppState>,
    req: HttpRequest,
) -> Result<ApiResponse<ReminderPrefsView>, ApiError> {
    let claims = crate::auth::user_claims(&req)?;
    let prefs = state.reminder_prefs.get(claims.user_id).await.map_err(internal)?;
    Ok(ApiResponse::ok(prefs.into()))
}

#[utoipa::path(
    put,
    path = "/api/v1/reminder-preferences",
    operation_id = "reminder_prefs_put",
    request_body = ReminderPrefsView,
    responses(
        (status = 200, description = "Settings saved", body = ReminderPrefsResponseBody),
        (status = 401, description = "No session"),
        (status = 422, description = "Validation failed"),
    ),
    security(("cookie_access" = [])),
    tag = "reminders",
)]
#[allow(clippy::future_not_send)]
#[allow(unreachable_pub)]
#[put("/reminder-preferences")]
pub async fn put_prefs(
    state: web::Data<AppState>,
    req: HttpRequest,
    body: web::Json<ReminderPrefsView>,
) -> Result<ApiResponse<ReminderPrefsView>, ApiError> {
    let claims = crate::auth::user_claims(&req)?;
    let b = body.into_inner();
    if b.lead_days < LEAD_DAYS_MIN || b.lead_days > LEAD_DAYS_MAX {
        return Err(ApiError::Validation(vec![FieldViolation::new(
            "/lead_days",
            "validation.out_of_range",
            "lead_days must be between 0 and 21",
        )
        .with_param("min", LEAD_DAYS_MIN)
        .with_param("max", LEAD_DAYS_MAX)]));
    }
    let prefs = ReminderPreferences {
        emails_enabled: b.emails_enabled,
        remind_birthdays: b.remind_birthdays,
        remind_anniversaries: b.remind_anniversaries,
        favourites_only: b.favourites_only,
        lead_days: b.lead_days,
    };
    state.reminder_prefs.upsert(claims.user_id, prefs).await.map_err(internal)?;
    Ok(ApiResponse::ok(prefs.into()))
}
