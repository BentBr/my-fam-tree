//! HTTP API. Public from the binary entry point; openapi crate consumes the `ApiDoc`.

pub mod config;
pub mod tracing_setup;

pub use config::{AppEnv, Config, ConfigError, LogFormat};
pub use tracing_setup::init_tracing;
