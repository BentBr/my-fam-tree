//! `audit_log` repository trait.
//!
//! The audit log captures every admin-visible mutation: contact CRUD,
//! person CRUD, parent-link / partnership writes, and family role
//! changes (invite acceptance, member removal, role updates). Failures
//! from `record` are intentionally swallowed by the API layer so an
//! audit-log hiccup never blocks the user's request.

use async_trait::async_trait;
use serde_json::Value;
use uuid::Uuid;

use crate::{FamilyId, UserId};

#[derive(Debug, Clone)]
pub struct AuditEntry {
    pub family_id: FamilyId,
    pub actor_user_id: Option<UserId>,
    pub action: String,
    pub entity_kind: String,
    pub entity_id: Option<Uuid>,
    pub metadata: Value,
}

#[derive(Debug, thiserror::Error)]
pub enum AuditRepoError {
    #[error("database: {0}")]
    Db(String),
}

#[async_trait]
pub trait AuditLogRepo: Send + Sync {
    async fn record(&self, entry: AuditEntry) -> Result<(), AuditRepoError>;
}
