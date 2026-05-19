//! HTTP API. Public from the binary entry point; openapi crate consumes the `ApiDoc`.

pub mod config;
pub mod error;
pub mod middleware;
pub mod response;
pub mod tracing_setup;

pub use config::{AppEnv, Config, ConfigError, LogFormat};
pub use error::{ApiError, ApiErrorBody, ApiResult, ErrorCode, FieldViolation};
pub use response::{ApiResponse, Pagination, ResponseMeta};
pub use tracing_setup::init_tracing;
