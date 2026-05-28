//! Janitor tick — one call into [`JanitorRepo::sweep_expired`].
//!
//! The main loop owns the cadence (`WORKER_JANITOR_INTERVAL_SECONDS`); this
//! module is the thin wrapper that builds the `grace` window from config,
//! invokes the sweep, and logs the per-table counts. Errors are logged and
//! swallowed — a sweep failure must NOT take down the worker loop.

use chrono::Duration;

use crate::state::WorkerState;

/// Run a single sweep against the configured janitor repo. Logs the result.
pub async fn run_sweep(state: &WorkerState) {
    let now = state.clock.now();
    let grace_secs = i64::try_from(state.janitor_grace_seconds).unwrap_or(i64::MAX);
    let grace = Duration::seconds(grace_secs);
    match state.janitor.sweep_expired(now, grace).await {
        Ok(report) if report.total() > 0 => tracing::info!(
            magic_links = report.magic_links_deleted,
            refresh_tokens = report.refresh_tokens_deleted,
            family_invites = report.family_invites_deleted,
            owner_transfers = report.owner_transfers_deleted,
            "janitor swept expired rows",
        ),
        Ok(_) => tracing::debug!("janitor sweep: nothing to delete"),
        Err(e) => tracing::error!(?e, "janitor sweep failed"),
    }
}
