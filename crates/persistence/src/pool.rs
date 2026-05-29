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
    /// Connect to Postgres at `url` and configure the pool.
    ///
    /// # Errors
    /// Returns [`PersistenceError::Config`] if the URL is unparseable, or
    /// [`PersistenceError::Sqlx`] if the server is unreachable.
    pub async fn connect(
        url: &str,
        max_connections: u32,
        acquire_timeout: Duration,
        statement_timeout_ms: u32,
    ) -> Result<Self, PersistenceError> {
        let mut opts = PgConnectOptions::from_str(url)
            .map_err(|e| PersistenceError::Config(e.to_string()))?
            .application_name("my-fam-tree");
        opts = opts.log_statements(tracing::log::LevelFilter::Debug);

        let pool = PgPoolOptions::new()
            .max_connections(max_connections)
            .acquire_timeout(acquire_timeout)
            .after_connect(move |conn, _meta| {
                // `SET statement_timeout = $1` is rejected by the parser — GUC
                // assignment statements don't take bind params. `set_config()`
                // is the documented Postgres function for "session SET with a
                // dynamic value", and it accepts a bind parameter.
                let timeout = statement_timeout_ms.to_string();
                Box::pin(async move {
                    sqlx::query("SELECT set_config('statement_timeout', $1, false)")
                        .bind(timeout)
                        .execute(conn)
                        .await?;
                    Ok(())
                })
            })
            .connect_with(opts)
            .await?;

        Ok(Self { pool })
    }

    #[must_use]
    pub const fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// Verify the connection by issuing `SELECT 1`.
    ///
    /// # Errors
    /// Returns [`PersistenceError::Sqlx`] if the query fails or the server is
    /// unreachable.
    pub async fn ping(&self) -> Result<(), PersistenceError> {
        let _: (i32,) = sqlx::query_as("SELECT 1").fetch_one(&self.pool).await?;
        Ok(())
    }
}
