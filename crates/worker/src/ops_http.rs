//! In-process actix listener for operational endpoints.
//!
//! Always serves `GET /health` (the compose / k8s probe surface — see
//! `health.rs` for the response contract). Under the `test-fixtures`
//! cargo feature the same listener also serves
//! `POST /__test/advance-clock` so the E2E suite can fast-forward the
//! worker's clock without waiting for the real 06:00 window.
//!
//! Bind comes from `WORKER_METRICS_BIND` (`0.0.0.0:9091` in the default
//! dev env). One listener serves both routes so there's only one port
//! the ops repo + k8s manifest need to expose.

use std::sync::Arc;

use actix_web::{App, HttpServer, web};

use crate::health::{HealthState, Heartbeat};
#[cfg(feature = "test-fixtures")]
use crate::{clock::OffsetClock, state::WorkerState};

/// Bind the configured `HttpServer` to `bind` and run it to completion.
/// Both `serve()` and `serve_with_test_fixtures()` share this tail, so
/// the bind-fail vs run-fail logging stays in one place. We take the
/// already-`.run()`-mapped result so the caller is free to construct
/// any `HttpServer` shape (the closure types differ between the two
/// variants and aren't worth naming generically).
async fn run_listener(listener: std::io::Result<actix_web::dev::Server>) {
    match listener {
        Ok(server) => {
            if let Err(e) = server.await {
                tracing::error!(?e, "worker ops listener stopped");
            }
        }
        Err(e) => tracing::error!(?e, "failed to bind worker ops listener"),
    }
}

/// Serve `/health` on `bind`. Runs until the process exits.
///
/// `stale_after_ms` is the staleness threshold the probe uses to decide
/// between 200 OK and 503: a heartbeat older than this counts as a
/// wedged loop. Pick something larger than the loop's natural cadence
/// (refresh interval) plus a small jitter margin — e.g. `2 *
/// leader_refresh_seconds * 1000` is a sane default.
pub async fn serve(heartbeat: Arc<Heartbeat>, stale_after_ms: i64, bind: String) {
    let state = web::Data::new(HealthState { heartbeat, stale_after_ms });
    tracing::info!(%bind, "worker ops listener starting (/health)");
    let server = HttpServer::new(move || {
        let app = App::new().app_data(state.clone()).service(crate::health::health);
        // The test-fixtures route is mounted on the SAME App so we only
        // bind one socket. The route lives behind the cargo feature so
        // a prod build can't accidentally expose it.
        #[cfg(feature = "test-fixtures")]
        let app = app.service(test_fixtures::advance);
        app
    });
    run_listener(server.bind(&bind).map(HttpServer::run)).await;
}

/// `test-fixtures`-only entry point: serve the ops listener with the
/// advance-clock fixture wired in. The `worker_state` + `fixed` clock
/// are injected via `app_data` alongside the health state.
#[cfg(feature = "test-fixtures")]
pub async fn serve_with_test_fixtures(
    heartbeat: Arc<Heartbeat>,
    stale_after_ms: i64,
    bind: String,
    worker_state: WorkerState,
    fixed: Arc<OffsetClock>,
) {
    let health_data = web::Data::new(HealthState { heartbeat, stale_after_ms });
    let test_data = web::Data::new(test_fixtures::TestState { state: worker_state, fixed });
    tracing::warn!(%bind, "worker ops listener starting WITH test-fixtures — DO NOT enable in production");
    let server = HttpServer::new(move || {
        App::new()
            .app_data(health_data.clone())
            .app_data(test_data.clone())
            .service(crate::health::health)
            .service(test_fixtures::advance)
    });
    run_listener(server.bind(&bind).map(HttpServer::run)).await;
}

#[cfg(feature = "test-fixtures")]
mod test_fixtures {
    //! Dev/E2E-only `POST /__test/advance-clock` handler. Moved here
    //! from the old `test_clock_http` module so a single actix listener
    //! can host both `/health` and the test-fixtures route.

    use std::sync::Arc;

    use actix_web::{HttpResponse, post, web};
    use chrono::{DateTime, NaiveDate, TimeZone, Utc};
    use chrono_tz::Europe::Berlin;
    use serde::Deserialize;

    use crate::clock::OffsetClock;
    use crate::state::WorkerState;
    use crate::ticker;

    #[derive(Debug, Deserialize)]
    struct AdvanceReq {
        /// Explicit UTC instant. Takes precedence over `date` when both are given.
        #[serde(default)]
        to: Option<DateTime<Utc>>,
        /// A calendar date, resolved to 06:00 Europe/Berlin (the worker's default
        /// user timezone), DST-aware. Defaults to today when neither field is set —
        /// so `{}` advances to "this morning's 06:00" and fires today's reminders.
        #[serde(default)]
        date: Option<NaiveDate>,
    }

    fn resolve_instant(req: &AdvanceReq) -> DateTime<Utc> {
        if let Some(to) = req.to {
            return to;
        }
        let date = req.date.unwrap_or_else(|| Utc::now().with_timezone(&Berlin).date_naive());
        let Some(naive) = date.and_hms_opt(6, 0, 0) else { return Utc::now() };
        Berlin
            .from_local_datetime(&naive)
            .single()
            .map_or_else(Utc::now, |dt| dt.with_timezone(&Utc))
    }

    #[derive(Clone)]
    pub(super) struct TestState {
        pub state: WorkerState,
        pub fixed: Arc<OffsetClock>,
    }

    #[post("/__test/advance-clock")]
    #[allow(
        unreachable_pub,
        reason = "the #[post] proc-macro replaces this fn with a unit struct; pub is required for actix's HttpServiceFactory wiring"
    )]
    pub(super) async fn advance(
        ts: web::Data<TestState>,
        body: web::Json<AdvanceReq>,
    ) -> HttpResponse {
        let to = resolve_instant(&body);
        ts.fixed.set(to);
        match ticker::run_tick(&ts.state).await {
            Ok(scheduled) => HttpResponse::Ok().json(serde_json::json!({
                "now": to,
                "scheduled": scheduled,
            })),
            Err(e) => HttpResponse::InternalServerError().body(format!("tick error: {e}")),
        }
    }
}
