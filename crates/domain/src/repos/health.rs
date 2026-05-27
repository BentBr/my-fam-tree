//! A cheap DB reachability + schema probe for the `/health` endpoint.

use async_trait::async_trait;

/// Errors surfaced by [`HealthRepo`].
#[derive(Debug, thiserror::Error)]
pub enum HealthRepoError {
    #[error("database: {0}")]
    Db(String),
}

#[async_trait]
pub trait HealthRepo: Send + Sync {
    /// Run a cheap query that proves the DB answered AND the core schema is
    /// present (a `SELECT` against a migrated table). `Ok(())` = reachable.
    ///
    /// # Errors
    /// Returns [`HealthRepoError::Db`] if the query fails (DB down, schema
    /// missing, connection exhausted).
    async fn ping(&self) -> Result<(), HealthRepoError>;
}
