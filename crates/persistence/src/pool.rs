use std::str::FromStr;
use std::time::Duration;

use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use sqlx::{ConnectOptions, PgPool};

use crate::error::PersistenceError;

#[derive(Clone, Debug)]
pub struct Database {
    pool: PgPool,
}

impl Database {
    pub async fn connect(
        url: &str,
        max_connections: u32,
        acquire_timeout: Duration,
        statement_timeout_ms: u32,
    ) -> Result<Self, PersistenceError> {
        let mut opts = PgConnectOptions::from_str(url)
            .map_err(|e| PersistenceError::Config(e.to_string()))?
            .application_name("my-family");
        opts = opts.log_statements(tracing::log::LevelFilter::Debug);

        let pool = PgPoolOptions::new()
            .max_connections(max_connections)
            .acquire_timeout(acquire_timeout)
            .after_connect(move |conn, _meta| {
                Box::pin(async move {
                    sqlx::query(&format!("SET statement_timeout = {statement_timeout_ms}"))
                        .execute(conn)
                        .await?;
                    Ok(())
                })
            })
            .connect_with(opts)
            .await?;

        Ok(Self { pool })
    }

    pub const fn pool(&self) -> &PgPool {
        &self.pool
    }

    pub async fn ping(&self) -> Result<(), PersistenceError> {
        let _: (i32,) = sqlx::query_as("SELECT 1").fetch_one(&self.pool).await?;
        Ok(())
    }
}
