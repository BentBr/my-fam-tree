//! `WorkerConfig` — everything the `worker` binary reads from the env.

use serde::Deserialize;

use crate::common::{
    AppEnv, DatabaseConfig, EmailConfig, LogConfig, LogFormat, RedisConfig, WebConfig,
};
use crate::storage::{StorageConfig, StorageDriver};
use crate::{ConfigError, load_flat};

/// Leader-loop tuning + retry envelope shared by the ticker + dispatcher.
#[derive(Debug, Clone)]
pub struct WorkerLoopConfig {
    pub tick_interval_seconds: u64,
    pub leader_lease_seconds: u64,
    pub leader_refresh_seconds: u64,
    pub max_retries: i32,
    pub retry_backoff_min_seconds: u64,
    pub retry_backoff_max_seconds: u64,
    pub metrics_bind: String,
}

/// Periodic janitor sweep — DELETEs expired auth / invite / transfer rows.
#[derive(Debug, Clone)]
pub struct JanitorConfig {
    pub interval_seconds: u64,
    pub grace_seconds: u64,
}

/// Outbox dispatcher pool — drains the durable `email_outbox` via SMTP.
#[derive(Debug, Clone)]
pub struct OutboxConfig {
    pub poll_seconds: u64,
    pub pool_size: usize,
}

/// What the `worker` binary loads from the environment.
#[derive(Debug, Clone)]
pub struct WorkerConfig {
    pub app_env: AppEnv,
    pub log: LogConfig,
    pub database: DatabaseConfig,
    pub redis: RedisConfig,
    pub email: EmailConfig,
    pub web: WebConfig,
    pub storage: StorageConfig,
    pub worker: WorkerLoopConfig,
    pub janitor: JanitorConfig,
    pub outbox: OutboxConfig,
}

// NB: NO `#[serde(deny_unknown_fields)]` — figment's `Env::raw()` merges the
// whole process environment (PATH, HOME, the shell's `_`, …) and the
// deserializer would reject every unknown env var the system happens to have.
// We rely on REQUIRED fields being absent to surface configuration mistakes.
#[derive(Deserialize)]
struct FlatWorkerConfig {
    app_env: AppEnv,
    rust_log: String,
    log_format: LogFormat,

    database_url: String,
    database_max_connections: u32,
    database_acquire_timeout_seconds: u64,
    database_statement_timeout_ms: u32,

    redis_url: String,
    redis_max_connections: usize,
    redis_key_prefix: String,

    email_dsn: String,
    email_from_name: String,
    email_from_address: String,
    email_reply_to: Option<String>,
    email_timeout_seconds: u64,

    web_public_url: String,

    storage_driver: StorageDriver,
    storage_bucket: String,
    storage_region: String,
    storage_endpoint_url: Option<String>,
    storage_access_key_id: String,
    storage_secret_access_key: String,
    storage_force_path_style: bool,

    worker_tick_interval_seconds: u64,
    worker_leader_lease_seconds: u64,
    worker_leader_refresh_seconds: u64,
    worker_max_retries: i32,
    worker_retry_backoff_min_seconds: u64,
    worker_retry_backoff_max_seconds: u64,
    worker_metrics_bind: String,

    worker_janitor_interval_seconds: u64,
    worker_janitor_grace_seconds: u64,

    worker_outbox_poll_seconds: u64,
    worker_outbox_pool_size: usize,
}

impl WorkerConfig {
    /// Load the worker configuration from the process environment.
    ///
    /// # Errors
    /// Returns [`ConfigError`] if any required env var is missing or
    /// fails to parse into its typed shape.
    pub fn from_env() -> Result<Self, ConfigError> {
        let f: FlatWorkerConfig = load_flat()?;
        Ok(Self {
            app_env: f.app_env,
            log: LogConfig { level: f.rust_log, format: f.log_format },
            database: DatabaseConfig {
                url: f.database_url,
                max_connections: f.database_max_connections,
                acquire_timeout_seconds: f.database_acquire_timeout_seconds,
                statement_timeout_ms: f.database_statement_timeout_ms,
            },
            redis: RedisConfig {
                url: f.redis_url,
                max_connections: f.redis_max_connections,
                key_prefix: f.redis_key_prefix,
            },
            email: EmailConfig {
                dsn: f.email_dsn,
                from_name: f.email_from_name,
                from_address: f.email_from_address,
                reply_to: f.email_reply_to,
                timeout_seconds: f.email_timeout_seconds,
            },
            web: WebConfig { public_url: f.web_public_url },
            storage: StorageConfig {
                driver: f.storage_driver,
                bucket: f.storage_bucket,
                region: f.storage_region,
                endpoint_url: f.storage_endpoint_url,
                access_key_id: f.storage_access_key_id,
                secret_access_key: f.storage_secret_access_key,
                force_path_style: f.storage_force_path_style,
            },
            worker: WorkerLoopConfig {
                tick_interval_seconds: f.worker_tick_interval_seconds,
                leader_lease_seconds: f.worker_leader_lease_seconds,
                leader_refresh_seconds: f.worker_leader_refresh_seconds,
                max_retries: f.worker_max_retries,
                retry_backoff_min_seconds: f.worker_retry_backoff_min_seconds,
                retry_backoff_max_seconds: f.worker_retry_backoff_max_seconds,
                metrics_bind: f.worker_metrics_bind,
            },
            janitor: JanitorConfig {
                interval_seconds: f.worker_janitor_interval_seconds,
                grace_seconds: f.worker_janitor_grace_seconds,
            },
            outbox: OutboxConfig {
                poll_seconds: f.worker_outbox_poll_seconds,
                pool_size: f.worker_outbox_pool_size,
            },
        })
    }
}
