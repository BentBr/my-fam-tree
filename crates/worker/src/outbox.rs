//! Outbox dispatcher pool.
//!
//! Drains `email_outbox` rows whose `next_attempt_at <= now` and SMTPs them
//! via the worker's [`EmailSender`]. Runs in parallel with the reminder
//! ticker + janitor under the same leader lock — actually, it does NOT
//! require the leader lock: `claim_next_due` already uses `FOR UPDATE
//! SKIP LOCKED`, so any number of pollers across any number of worker
//! replicas can drain without seeing the same row. Keeping it lock-free
//! also means a brand-new worker can clear a backlog faster than the
//! single-leader ticker can.

use std::time::Duration;

use my_family_email::OutboundEmail;

use crate::backoff;
use crate::state::WorkerState;

/// Process ONE due email. Returns `true` if a row was claimed (caller may
/// loop tight), `false` if nothing was due (caller should sleep).
pub async fn process_one(state: &WorkerState) -> bool {
    let now = state.clock.now();
    match state.outbox.claim_next_due(now).await {
        Ok(None) => false,
        Ok(Some(row)) => {
            let email = OutboundEmail {
                to_addr: row.to_addr.clone(),
                to_name: None,
                subject: row.subject.clone(),
                text_body: row.text_body.clone(),
                html_body: row.html_body.clone(),
            };
            match state.email.send(email).await {
                Ok(()) => {
                    if let Err(e) = state.outbox.mark_sent(row.id, state.clock.now()).await {
                        tracing::error!(?e, outbox_id = %row.id.into_uuid(), "outbox mark_sent failed");
                    } else {
                        tracing::info!(
                            outbox_id = %row.id.into_uuid(),
                            kind = %row.kind,
                            to_addr = %row.to_addr,
                            "outbox email sent",
                        );
                    }
                }
                Err(e) => {
                    let next_attempts = row.attempts + 1;
                    let err_msg = e.to_string();
                    if next_attempts >= state.max_retries {
                        if let Err(me) =
                            state.outbox.mark_failed_permanent(row.id, &err_msg).await
                        {
                            tracing::error!(
                                ?me,
                                outbox_id = %row.id.into_uuid(),
                                "outbox mark_failed_permanent failed",
                            );
                        } else {
                            tracing::warn!(
                                outbox_id = %row.id.into_uuid(),
                                kind = %row.kind,
                                attempts = next_attempts,
                                error = %err_msg,
                                "outbox email permanently failed",
                            );
                        }
                    } else {
                        let next_at = backoff::next_attempt(
                            state.clock.now(),
                            row.attempts,
                            state.retry_min_seconds,
                            state.retry_max_seconds,
                        );
                        if let Err(me) = state.outbox.mark_retry(row.id, next_at, &err_msg).await
                        {
                            tracing::error!(
                                ?me,
                                outbox_id = %row.id.into_uuid(),
                                "outbox mark_retry failed",
                            );
                        } else {
                            tracing::info!(
                                outbox_id = %row.id.into_uuid(),
                                kind = %row.kind,
                                attempts = next_attempts,
                                next_at = %next_at,
                                error = %err_msg,
                                "outbox email retry scheduled",
                            );
                        }
                    }
                }
            }
            true
        }
        Err(e) => {
            tracing::error!(?e, "outbox claim_next_due failed");
            false
        }
    }
}

/// Long-running poller — one of N parallel tasks. Tight-loops while there's
/// work, sleeps `poll_interval` when the queue is empty.
pub async fn run_poller(state: WorkerState, poll_interval: Duration) {
    loop {
        let did_work = process_one(&state).await;
        if !did_work {
            tokio::time::sleep(poll_interval).await;
        }
    }
}
