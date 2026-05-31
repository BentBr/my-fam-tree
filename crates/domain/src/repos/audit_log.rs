//! `audit_log` repository trait.
//!
//! The audit log captures every admin-visible mutation: contact CRUD,
//! person CRUD, parent-link / partnership writes, and family role
//! changes (invite acceptance, member removal, role updates). Failures
//! from `record` are intentionally swallowed by the API layer so an
//! audit-log hiccup never blocks the user's request.
//!
//! [`AuditLogRepo::list_filtered`] is the admin-only read side: it
//! returns paged rows joined with the actor user (display name + email)
//! and a resolved `entity_person_id` so the FE can deep-link straight
//! into `/tree?center=<person>`. The CASE mapping that derives that
//! person from each row's `entity_kind` + `metadata` lives in the
//! Postgres implementation — keeping the SQL inside `crates/persistence`
//! is an architectural rule.

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

/// Filter for [`AuditLogRepo::list_filtered`].
///
/// All optional fields mean "do not constrain on this dimension".
/// `from` / `to` are inclusive on both sides. `page` is 1-based;
/// `page_size` is clamped by the persistence impl to the supported
/// set (50 / 100 / 200 / 500) so callers can't blow the query plan.
#[derive(Debug, Clone)]
pub struct AuditFilter {
    pub family_id: FamilyId,
    pub from: Option<chrono::DateTime<chrono::Utc>>,
    pub to: Option<chrono::DateTime<chrono::Utc>>,
    pub action: Option<String>,
    pub entity_kind: Option<String>,
    pub actor_user_id: Option<UserId>,
    pub page: u32,
    pub page_size: u32,
}

/// One audit-log row with the actor and entity-person already resolved.
///
/// `entity_person_id` is filled when the persistence query can derive a
/// single person from `entity_kind` + `metadata`. `entity_person_name`
/// is `given_name + ' ' + family_name` when a name is available.
#[derive(Debug, Clone)]
pub struct AuditRow {
    pub id: Uuid,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub action: String,
    pub entity_kind: String,
    pub entity_id: Option<Uuid>,
    pub metadata: Value,
    pub actor_user_id: Option<UserId>,
    pub actor_display_name: Option<String>,
    pub actor_email: Option<String>,
    /// Name of the `persons` row (in this family) the actor is
    /// linked to via `linked_user_id`, if any. The FE falls back to
    /// this when the actor's account `display_name` is empty so a
    /// brand-new member's audit rows still surface a meaningful
    /// name (the family already knows them by the linked-person
    /// name).
    pub actor_person_name: Option<String>,
    pub entity_person_id: Option<Uuid>,
    pub entity_person_name: Option<String>,
}

/// Page meta returned alongside the row batch.
#[derive(Debug, Clone, Copy)]
pub struct AuditPageMeta {
    pub total: i64,
}

#[derive(Debug, thiserror::Error)]
pub enum AuditRepoError {
    #[error("database: {0}")]
    Db(String),
}

#[async_trait]
pub trait AuditLogRepo: Send + Sync {
    async fn record(&self, entry: AuditEntry) -> Result<(), AuditRepoError>;

    /// Page of audit rows for the given filter, plus the total count
    /// of matching rows (so the FE paginator knows how many pages
    /// exist without a second round-trip).
    async fn list_filtered(
        &self,
        filter: AuditFilter,
    ) -> Result<(Vec<AuditRow>, AuditPageMeta), AuditRepoError>;
}
