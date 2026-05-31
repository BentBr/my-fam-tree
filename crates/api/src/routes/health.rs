//! `GET /api/v1/health` — liveness + DB-reachability probe.
//!
//! Always returns HTTP 200 (so a status page renders and a container
//! healthcheck reflects "API process is up") and conveys DB state in the
//! body: `db_ok` plus the measured `db_latency_ms`, and the
//! `server_duration_ms` covering the entire handler so the FE can
//! distinguish DB-bound slowness from API-bound slowness. The FE
//! colours each latency on its own thresholds (green < 100 ms,
//! yellow < 200 ms, red ≥ 200 ms; an unreachable DB is always red).

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
    /// Total handler duration in milliseconds. Includes the DB probe
    /// plus any framework / serialization overhead inside actix-web.
    /// Useful for distinguishing "DB is slow" (`db_latency_ms` close
    /// to `server_duration_ms`) from "API is slow elsewhere" (the gap
    /// between the two is dominated by other work).
    pub server_duration_ms: u64,
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
    // Two timers: the outer one covers the whole handler (returned as
    // `server_duration_ms`), the inner one isolates the DB probe so
    // callers can tell which part is slow. The DB probe runs INSIDE
    // the outer timer's window, so `server_duration_ms` is always
    // >= `db_latency_ms`.
    let started_total = Instant::now();
    let rid = req.extensions().get::<RequestIdValue>().map(|v| v.0.clone());

    // Time the DB probe. A failure is NOT an error response — we report it in
    // the body and keep the endpoint at 200 (the API process is alive).
    let started_db = Instant::now();
    let db_ok = state.health.ping().await.is_ok();
    let db_latency_ms = u64::try_from(started_db.elapsed().as_millis()).unwrap_or(u64::MAX);

    let server_duration_ms =
        u64::try_from(started_total.elapsed().as_millis()).unwrap_or(u64::MAX);

    let mut resp = ApiResponse::ok(Health {
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
        db_ok,
        db_latency_ms,
        server_duration_ms,
    });
    if let Some(rid) = rid {
        resp = resp.with_request_id(rid);
    }
    Ok(resp)
}
