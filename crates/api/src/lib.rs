//! HTTP API. Public from the binary entry point; openapi crate consumes the `ApiDoc`.

pub mod config;

pub use config::{AppEnv, Config, ConfigError, LogFormat};
