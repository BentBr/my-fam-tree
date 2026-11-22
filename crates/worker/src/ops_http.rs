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
#[cfg(feature = "test-fixtures")]
use my_fam_tree_domain::clock::OffsetClock;

use crate::health::{HealthState, Heartbeat};
#[cfg(feature = "test-fixtures")]
use crate::state::WorkerState;

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
    zones: Arc<dyn my_fam_tree_domain::TimeZoneRepo>,
) {
    let health_data = web::Data::new(HealthState { heartbeat, stale_after_ms });
    let test_data = web::Data::new(test_fixtures::TestState { state: worker_state, fixed, zones });
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
    use my_fam_tree_domain::TimeZoneRepo;
    use my_fam_tree_domain::clock::OffsetClock;
    use my_fam_tree_domain::dates::{DEFAULT_TIMEZONE, DIGEST_LOCAL_HOUR};
    use serde::Deserialize;
    use time::format_description::well_known::Rfc3339;
    use time::{Date, OffsetDateTime, Time};

    use crate::state::WorkerState;
    use crate::ticker;

    #[derive(Debug, Deserialize)]
    struct AdvanceReq {
        /// Explicit UTC instant. Takes precedence over `date` when both are given.
        #[serde(default, with = "time::serde::rfc3339::option")]
        to: Option<OffsetDateTime>,
        /// A calendar date, resolved to 06:00 Europe/Berlin (the worker's default
        /// user timezone), DST-aware. Defaults to today when neither field is set —
        /// so `{}` advances to "this morning's 06:00" and fires today's reminders.
        #[serde(default)]
        date: Option<Date>,
    }

    /// `to` verbatim, else 06:00 in [`DEFAULT_TIMEZONE`] on `date` (default:
    /// today there). The zone math runs in Postgres via [`TimeZoneRepo`].
    async fn resolve_instant(
        req: &AdvanceReq,
        zones: &dyn TimeZoneRepo,
    ) -> Result<OffsetDateTime, my_fam_tree_domain::TimeZoneRepoError> {
        if let Some(to) = req.to {
            return Ok(to);
        }
        let date = match req.date {
            Some(d) => d,
            #[expect(
                clippy::disallowed_methods,
                reason = "an omitted date means today in real time, independent of where the fixture clock was moved"
            )]
            None => zones.local_date(OffsetDateTime::now_utc(), DEFAULT_TIMEZONE).await?,
        };
        let six = Time::from_hms(DIGEST_LOCAL_HOUR, 0, 0).unwrap_or(Time::MIDNIGHT);
        zones.local_to_utc(date, six, DEFAULT_TIMEZONE).await
    }

    #[derive(Clone)]
    pub(super) struct TestState {
        pub state: WorkerState,
        pub fixed: Arc<OffsetClock>,
        pub zones: Arc<dyn TimeZoneRepo>,
    }

    #[post("/__test/advance-clock")]
    pub(super) async fn advance(
        ts: web::Data<TestState>,
        body: web::Json<AdvanceReq>,
    ) -> HttpResponse {
        let to = match resolve_instant(&body, ts.zones.as_ref()).await {
            Ok(to) => to,
            Err(e) => {
                return HttpResponse::InternalServerError().body(format!("resolve error: {e}"));
            }
        };
        ts.fixed.set(to);
        match ticker::run_tick(&ts.state).await {
            Ok(scheduled) => HttpResponse::Ok().json(serde_json::json!({
                "now": to.format(&Rfc3339).unwrap_or_default(),
                "scheduled": scheduled,
            })),
            Err(e) => HttpResponse::InternalServerError().body(format!("tick error: {e}")),
        }
    }
}
