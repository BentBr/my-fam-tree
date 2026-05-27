//! Dev/E2E-only `POST /__test/advance-clock` listener.
//!
//! Compiled solely behind the `test-fixtures` cargo feature. Lets the E2E
//! suite fast-forward the worker's clock and run a tick immediately, instead
//! of waiting for the real 06:00 window. MUST NOT be enabled in production.

use std::sync::Arc;

use actix_web::{App, HttpResponse, HttpServer, post, web};
use chrono::{DateTime, Utc};
use serde::Deserialize;

use crate::clock::FixedClock;
use crate::state::WorkerState;
use crate::ticker;

#[derive(Debug, Deserialize)]
struct AdvanceReq {
    to: DateTime<Utc>,
}

#[derive(Clone)]
struct TestState {
    state: WorkerState,
    fixed: Arc<FixedClock>,
}

#[post("/__test/advance-clock")]
async fn advance(ts: web::Data<TestState>, body: web::Json<AdvanceReq>) -> HttpResponse {
    ts.fixed.set(body.to);
    match ticker::run_tick(&ts.state).await {
        Ok(scheduled) => HttpResponse::Ok().json(serde_json::json!({
            "now": body.to,
            "scheduled": scheduled,
        })),
        Err(e) => HttpResponse::InternalServerError().body(format!("tick error: {e}")),
    }
}

/// Serve the test-fixtures endpoint on `bind`. Runs until the process exits.
pub async fn serve(state: WorkerState, fixed: Arc<FixedClock>, bind: String) {
    let data = web::Data::new(TestState { state, fixed });
    tracing::warn!(%bind, "test-fixtures HTTP listener enabled — DO NOT enable in production");
    match HttpServer::new(move || App::new().app_data(data.clone()).service(advance)).bind(&bind) {
        Ok(server) => {
            if let Err(e) = server.run().await {
                tracing::error!(?e, "test-fixtures listener stopped");
            }
        }
        Err(e) => tracing::error!(?e, "failed to bind test-fixtures listener"),
    }
}
