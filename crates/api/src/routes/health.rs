//! `GET /api/v1/health` — liveness + DB-reachability probe.
//!
//! Always returns HTTP 200 (so a status page renders and a container
//! healthcheck reflects "API process is up") and conveys DB state in the body:
//! `db_ok` plus the measured `db_latency_ms`. The FE colours the latency
//! (green < 100 ms, yellow < 200 ms, red ≥ 200 ms or `db_ok: false`).

use std::time::Instant;

use actix_web::{HttpMessage, get, web};
use serde::Serialize;
use utoipa::ToSchema;

use crate::middleware::RequestIdValue;
use crate::{ApiError, ApiResponse, AppState, response_body};

#[derive(Debug, Serialize, ToSchema)]
pub struct Health {
    pub status: &'static str,
    pub version: &'static str,
    /// `true` when the DB answered the reachability probe.
    pub db_ok: bool,
    /// Round-trip duration of the DB probe in milliseconds.
    pub db_latency_ms: u64,
}

response_body!(pub HealthResponseBody, Health);

#[utoipa::path(
    get,
    path = "/api/v1/health",
    responses(
        (status = 200, description = "Service is healthy", body = HealthResponseBody),
    ),
    tag = "health",
)]
// `actix_web::HttpRequest` holds an `Rc`, so the returned future is `!Send`;
// this is the canonical actix-web handler signature.
#[allow(clippy::future_not_send)]
// The `#[get("/health")]` proc-macro replaces this function with a `struct health`
// that implements `HttpServiceFactory`, which trips `unreachable_pub` on the fn.
// The `pub` is needed so the `openapi` crate can name it in `paths(...)`.
#[allow(unreachable_pub)]
#[get("/health")]
pub async fn health(
    state: web::Data<AppState>,
    req: actix_web::HttpRequest,
) -> Result<ApiResponse<Health>, ApiError> {
    let rid = req.extensions().get::<RequestIdValue>().map(|v| v.0.clone());

    // Time the DB probe. A failure is NOT an error response — we report it in
    // the body and keep the endpoint at 200 (the API process is alive).
    let started = Instant::now();
    let db_ok = state.health.ping().await.is_ok();
    let db_latency_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);

    let mut resp = ApiResponse::ok(Health {
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
        db_ok,
        db_latency_ms,
    });
    if let Some(rid) = rid {
        resp = resp.with_request_id(rid);
    }
    Ok(resp)
}
