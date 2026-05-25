//! Postgres-backed [`AuditLogRepo`] implementation.

use async_trait::async_trait;
use my_family_domain::{AuditEntry, AuditLogRepo, AuditRepoError};
use sqlx::PgPool;

#[derive(Clone, Debug)]
pub struct PgAuditLogRepo {
    pool: PgPool,
}

impl PgAuditLogRepo {
    #[must_use]
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl AuditLogRepo for PgAuditLogRepo {
    async fn record(&self, entry: AuditEntry) -> Result<(), AuditRepoError> {
        let metadata = entry.metadata;
        sqlx::query!(
            r#"INSERT INTO audit_log (family_id, actor_user_id, action, entity_kind, entity_id, metadata)
               VALUES ($1, $2, $3, $4, $5, $6)"#,
            entry.family_id.into_uuid(),
            entry.actor_user_id.map(my_family_domain::UserId::into_uuid),
            entry.action,
            entry.entity_kind,
            entry.entity_id,
            metadata,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| AuditRepoError::Db(e.to_string()))?;
        Ok(())
    }
}
