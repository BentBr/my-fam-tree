//! `GET /api/v1/health` — liveness + DB + worker probe.
//!
//! Always returns HTTP 200 (so a status page renders and a container
//! healthcheck reflects "API process is up") and conveys subsystem
//! state in the body:
//!   - `db_ok` / `db_latency_ms` — Postgres reachability + probe RTT.
//!   - `worker_ok` — whether the worker currently holds its Redis
//!     leader-lease (`<prefix>reminder:leader`, set with a ~30 s TTL
//!     by `worker/src/leader.rs`). The lease auto-expires shortly
//!     after the worker dies, so a missing key reliably signals "no
//!     worker is processing the outbox / digests right now".
//!   - `server_duration_ms` — entire handler duration so the FE can
//!     distinguish DB-bound slowness from API-bound slowness.
//!
//! The FE colours each latency on its own thresholds (green < 100 ms,
//! yellow < 200 ms, red ≥ 200 ms); the DB / worker chips also flip
//! to red when the underlying `_ok` is false.

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
    /// `true` when the worker currently holds its Redis leader-lease
    /// (`<prefix>reminder:leader`). The lease has a short TTL and is
    /// refreshed each tick, so this signal flips false within ~30 s
    /// of the worker crashing — a fast "no worker is processing
    /// outbox / digests right now" readout for the status page.
    pub worker_ok: bool,
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

    // Worker liveness — does it currently hold the Redis leader lease?
    // Redis-unreachable maps to `worker_ok: false` (worker liveness
    // can't be confirmed → treat as down) rather than failing the
    // whole endpoint, so the API health page stays useful even when
    // Redis itself is the wedge.
    let worker_ok = worker_leader_alive(&state).await;

    let server_duration_ms = u64::try_from(started_total.elapsed().as_millis()).unwrap_or(u64::MAX);

    let mut resp = ApiResponse::ok(Health {
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
        db_ok,
        db_latency_ms,
        worker_ok,
        server_duration_ms,
    });
    if let Some(rid) = rid {
        resp = resp.with_request_id(rid);
    }
    Ok(resp)
}

/// Probe whether a worker currently holds the leader lease. The suffix
/// matches `worker/src/leader.rs::Leader::new` exactly — the lease key
/// is `<redis-prefix>reminder:leader`. Wrong suffix = false-negative
/// → treat as down.
///
/// Any Redis error (pool exhausted, unreachable host) collapses to
/// `false` — the status page would rather show a worker-down chip
/// than fail the whole health check.
async fn worker_leader_alive(state: &AppState) -> bool {
    state.redis.exists("reminder:leader").await.unwrap_or(false)
}
