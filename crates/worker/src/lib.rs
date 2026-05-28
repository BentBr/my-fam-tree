//! Reminder worker library.
//!
//! The leader-locked digest scheduler + dispatcher, factored into a lib so
//! integration tests can drive the ticker/dispatcher directly. The
//! `worker` binary (`main.rs`) is a thin wrapper that wires
//! production collaborators and runs the loop.

// The worker's async orchestration fns capture `Arc<dyn Repo>` trait objects;
// clippy's conservative `future_not_send` flags them even though the loops run
// on the multi-thread runtime fine. These are internal, not a library API
// contract, so allow crate-wide rather than tagging every fn (matches how the
// api crate handles the same lint per-handler).
#![allow(clippy::future_not_send)]

pub mod backoff;
pub mod clock;
pub mod config;
pub mod digest;
pub mod dispatcher;
pub mod leader;
pub mod state;
#[cfg(feature = "test-fixtures")]
pub mod test_clock_http;
pub mod ticker;
