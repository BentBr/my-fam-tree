//! Environment configuration for the reminder worker (parsed via `envy`).

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub app_env: String,
    pub rust_log: String,
    pub log_format: String,
    pub database_url: String,
    pub database_max_connections: u32,
    pub database_acquire_timeout_seconds: u64,
    pub database_statement_timeout_ms: u32,
    pub redis_url: String,
    pub redis_max_connections: usize,
    pub redis_key_prefix: String,
    pub email_dsn: String,
    pub email_from_name: String,
    pub email_from_address: String,
    pub email_reply_to: Option<String>,
    pub email_timeout_seconds: u64,
    pub web_public_url: String,
    pub worker_tick_interval_seconds: u64,
    pub worker_leader_lease_seconds: u64,
    pub worker_leader_refresh_seconds: u64,
    pub worker_max_retries: i32,
    pub worker_retry_backoff_min_seconds: u64,
    pub worker_retry_backoff_max_seconds: u64,
    #[serde(default = "default_metrics_bind")]
    pub worker_metrics_bind: String,
}

fn default_metrics_bind() -> String {
    "0.0.0.0:9091".to_string()
}

impl Config {
    /// Load from the process environment.
    ///
    /// # Errors
    /// Returns an error if a required variable is missing or unparsable.
    pub fn from_env() -> anyhow::Result<Self> {
        Ok(envy::from_env()?)
    }
}
