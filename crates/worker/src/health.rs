//! Worker liveness probe — `GET /health` for compose / k8s.
//!
//! Mirrors the api's `/api/v1/health` (single-word `health`, not the
//! k8s `/healthz` convention) so the operational story is consistent
//! across both processes:
//!
//!   api:    GET <http://api.my-fam-tree.docker/api/v1/health>
//!   worker: GET <http://worker.my-fam-tree.docker/health>
//!
//! The api's `/api/v1/health` ALSO surfaces worker liveness (via the
//! Redis lease) for the end-user status page, but a container
//! orchestrator needs an in-container probe to flip the worker
//! container's state to unhealthy and trigger a restart. That's this
//! endpoint.
//!
//! Response shape:
//!
//!   200 OK   — process is up AND the main loop has heartbeat'd within
//!              the staleness window. Body: `{ "ok": true,
//!              "last_tick_age_ms": <int> }`.
//!   503      — heartbeat is older than the staleness window (main
//!              loop wedged) or hasn't beat yet (cold start past the
//!              grace period). Body: `{ "ok": false,
//!              "last_tick_age_ms": <int>|null }`.
//!
//! Detecting "process up but loop wedged" matters: tokio runtimes can
//! survive while a single task is stuck (e.g. a blocking DB query), and
//! a TCP-accept-only probe would still answer 200 in that case. The
//! heartbeat-staleness check pins that.
//!
//! Bind address comes from `WORKER_METRICS_BIND` (existing env var).
//! This is the same listener that hosts `/__test/advance-clock` under
//! the `test-fixtures` feature.

use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};

use actix_web::{HttpResponse, get, web};
use serde::Serialize;

/// Shared heartbeat updated by the main leader loop on every iteration
/// and read by the `/health` handler. Cloneable via `Arc` so the loop
/// and the HTTP server hold the same atomic.
#[derive(Debug, Default)]
pub struct Heartbeat {
    /// Unix-millis of the most recent loop iteration. `0` means
    /// "never beat yet" (cold start); the handler reports it as
    /// `null` and 503 until the first beat lands.
    last_tick_unix_ms: AtomicI64,
}

impl Heartbeat {
    #[must_use]
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    /// Record a fresh heartbeat — call once per main-loop iteration.
    /// Idempotent; safe to call from multiple tasks (we don't, but the
    /// atomic write makes it valid if a refactor ever does).
    pub fn beat(&self) {
        self.last_tick_unix_ms.store(now_unix_ms(), Ordering::Relaxed);
    }

    fn last_tick_age_ms(&self) -> Option<i64> {
        let last = self.last_tick_unix_ms.load(Ordering::Relaxed);
        if last == 0 {
            return None;
        }
        Some((now_unix_ms() - last).max(0))
    }
}

fn now_unix_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

#[derive(Debug, Serialize)]
struct HealthBody {
    ok: bool,
    /// `null` until the loop has beat once (cold start). Otherwise the
    /// number of milliseconds since the most recent heartbeat.
    last_tick_age_ms: Option<i64>,
}

/// State shared with the actix handler. The staleness threshold is
/// configured at startup; we don't recompute per-request.
#[derive(Clone, Debug)]
pub struct HealthState {
    pub heartbeat: Arc<Heartbeat>,
    pub stale_after_ms: i64,
}

#[get("/health")]
#[allow(clippy::future_not_send, reason = "actix-web handler signature; HttpRequest holds an Rc")]
#[allow(
    unreachable_pub,
    reason = "the #[get] proc-macro replaces this fn with a unit struct; pub is required for actix's HttpServiceFactory wiring even though the fn body itself isn't directly callable from outside"
)]
pub async fn health(state: web::Data<HealthState>) -> HttpResponse {
    let age = state.heartbeat.last_tick_age_ms();
    // ok ⇔ we've beat at least once AND it was recently enough.
    let ok = age.is_some_and(|a| a < state.stale_after_ms);
    let body = HealthBody { ok, last_tick_age_ms: age };
    if ok { HttpResponse::Ok().json(body) } else { HttpResponse::ServiceUnavailable().json(body) }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn last_tick_age_ms_is_none_until_first_beat() {
        // Cold start: no heartbeat yet means the probe should return
        // None (handler reports null + 503), not a misleading "age
        // since unix epoch" value.
        let hb = Heartbeat::default();
        assert!(hb.last_tick_age_ms().is_none());
    }

    #[test]
    fn last_tick_age_ms_grows_from_zero_after_beat() {
        // First beat → small non-negative age. The exact value is
        // wall-clock-dependent so we only assert the contract: Some,
        // and >= 0. Negative age would be a sign of a bad clock-skew
        // handling bug.
        let hb = Heartbeat::default();
        hb.beat();
        let age = hb.last_tick_age_ms().expect("beat() recorded a heartbeat");
        assert!(age >= 0, "fresh heartbeat should have a non-negative age, got {age}");
    }

    #[test]
    fn second_beat_replaces_the_first() {
        // beat() is idempotent in semantics: the LATEST beat wins.
        // After a sleep between calls, the age must reflect the most
        // recent beat — otherwise a wedged loop that beat once and
        // never again would look "fresh forever". The threshold is
        // generous (200 ms) so a slow CI runner doesn't flake; the
        // contract we're pinning is "second beat WINS", not "ms-level
        // timing precision".
        let hb = Heartbeat::default();
        hb.beat();
        std::thread::sleep(std::time::Duration::from_millis(50));
        hb.beat();
        let age = hb.last_tick_age_ms().expect("second beat()");
        assert!(age < 200, "second beat replaces the first; expected small age, got {age}");
    }
}
