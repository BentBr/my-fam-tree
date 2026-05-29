//! Thin convenience wrapper around [`AuditLogRepo::record`].
//!
//! Every admin-visible mutation in the API (contact CRUD, person CRUD,
//! `parent_link` writes, partnership writes, family role changes)
//! goes through this helper. The repo error is **silently swallowed**
//! — an audit-log hiccup must never block the user's request. Failures
//! are logged via `tracing` at `warn!` for observability without
//! escalating to the client.

use std::sync::Arc;

use my_fam_tree_domain::{AuditEntry, AuditLogRepo, FamilyId, UserId};
use serde_json::Value;
use uuid::Uuid;

/// Record an audit entry, swallowing any repo error.
///
/// Caller passes the active `family_id`, the acting user's id, a short
/// `action` slug (`create` / `update` / `delete` / `invite` / etc.),
/// the affected `entity_kind` (`contact`, `person`, `parent_link`,
/// `partnership`, `membership`), an optional `entity_id`, and a
/// JSON `metadata` blob with whatever extra context belongs in the row.
pub async fn record(
    repo: &Arc<dyn AuditLogRepo>,
    family_id: FamilyId,
    actor: UserId,
    action: &str,
    entity_kind: &str,
    entity_id: Option<Uuid>,
    metadata: Value,
) {
    let entry = AuditEntry {
        family_id,
        actor_user_id: Some(actor),
        action: action.to_string(),
        entity_kind: entity_kind.to_string(),
        entity_id,
        metadata,
    };
    if let Err(err) = repo.record(entry).await {
        tracing::warn!(
            error = %err,
            action,
            entity_kind,
            "audit log write failed; continuing",
        );
    }
}
