//! Repository rules enforced as tests.
//!
//! Each module holds pure checker functions that take repo content and
//! return [`violation::Violation`]s. The integration tests under `tests/`
//! run them against the real repository, so `cargo test --workspace`
//! (pre-push + CI) enforces them.

pub mod deps;
pub mod docs;
pub mod refs;
pub mod repo;
pub mod rules;
pub mod sql;
pub mod violation;
