//! `tracing_subscriber` init for the API and worker binaries.

use my_family_config::LogFormat;
use tracing_subscriber::prelude::*;
use tracing_subscriber::{EnvFilter, fmt};

/// Initialise the global `tracing` subscriber.
///
/// `filter` is parsed as an `EnvFilter` directive (`RUST_LOG`-style). Falls back
/// to `"info"` if the filter is invalid. `format` selects pretty (dev) or JSON
/// (prod) output.
pub fn init_tracing(format: LogFormat, filter: &str) {
    let env_filter = EnvFilter::try_new(filter).unwrap_or_else(|_| EnvFilter::new("info"));
    let registry = tracing_subscriber::registry().with(env_filter);
    match format {
        LogFormat::Json => {
            registry.with(fmt::layer().json().with_current_span(true).with_span_list(true)).init();
        }
        LogFormat::Pretty => {
            registry.with(fmt::layer().pretty()).init();
        }
    }
}
