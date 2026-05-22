//! Service layer: orchestration between repos and HTTP handlers.
//!
//! Each module here exposes pure-async functions that take repo trait objects
//! and return domain types or anyhow errors; handlers translate those into
//! `ApiError` and `ApiResponse`.

pub mod auth_service;
pub mod relationships_tree;
pub mod upcoming;
