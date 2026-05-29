//! Postgres-backed [`AuditLogRepo`] implementation.
//!
//! The write side is a single `INSERT`; the read side runs a paginated
//! filter query that LEFT JOINs `users` for the actor display name and
//! resolves a per-row `entity_person_id` via a `CASE` over `entity_kind`.
//! The mapping is the source of truth for "which person does this audit
//! row link back to":
//!
//! - `person`         → `entity_id` is already the person id
//! - `contact`        → `metadata.person_id`
//! - `parent_link`    → `metadata.child_id`
//! - `partnership`    → `metadata.a`
//! - `membership`     → `persons.id WHERE family_id = al.family_id
//!                       AND linked_user_id = COALESCE(metadata.user_id,
//!                       actor_user_id)`
//! - `invite`         → `metadata.person_id` (Phase D writes this)
//! - everything else  → NULL (the FE renders a plain entity-kind chip)
//!
//! `COUNT(*) OVER()` returns the total matching count alongside the page
//! rows so the FE paginator never needs a second round-trip.

use async_trait::async_trait;
use my_fam_tree_domain::{
    AuditEntry, AuditFilter, AuditLogRepo, AuditPageMeta, AuditRepoError, AuditRow, UserId,
};
use serde_json::Value;
use sqlx::{PgPool, Row};
use uuid::Uuid;

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
            entry.actor_user_id.map(my_fam_tree_domain::UserId::into_uuid),
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

    #[allow(
        clippy::too_many_lines,
        reason = "single dynamic-filter query — splitting would obscure the SQL"
    )]
    async fn list_filtered(
        &self,
        filter: AuditFilter,
    ) -> Result<(Vec<AuditRow>, AuditPageMeta), AuditRepoError> {
        let page = filter.page.max(1);
        let page_size = match filter.page_size {
            50 | 100 | 200 | 500 => filter.page_size,
            _ => 50,
        };
        let offset: i64 = i64::from(page.saturating_sub(1)) * i64::from(page_size);
        let limit: i64 = i64::from(page_size);

        // Dynamic `sqlx::query` (not `query!`) so the optional filter
        // arguments don't have to be expanded into a Cartesian product
        // of compile-time variants. All inputs are bound; no string
        // concatenation of user data.
        let rows = sqlx::query(
            r"
            WITH filtered AS (
                SELECT
                    al.id,
                    al.created_at,
                    al.action,
                    al.entity_kind,
                    al.entity_id,
                    al.metadata,
                    al.actor_user_id,
                    u.display_name AS actor_display_name,
                    u.email::text  AS actor_email,
                    CASE al.entity_kind
                        WHEN 'person'       THEN al.entity_id
                        WHEN 'contact'      THEN (al.metadata->>'person_id')::uuid
                        WHEN 'parent_link'  THEN (al.metadata->>'child_id')::uuid
                        WHEN 'partnership'  THEN (al.metadata->>'a')::uuid
                        WHEN 'membership'   THEN (
                            SELECT id FROM persons
                            WHERE family_id = al.family_id
                              AND linked_user_id = COALESCE(
                                  (al.metadata->>'user_id')::uuid,
                                  al.actor_user_id
                              )
                            LIMIT 1
                        )
                        WHEN 'invite'       THEN (al.metadata->>'person_id')::uuid
                        ELSE NULL
                    END AS entity_person_id,
                    COUNT(*) OVER() AS total_count
                FROM audit_log al
                LEFT JOIN users u ON u.id = al.actor_user_id
                WHERE al.family_id = $1
                  AND ($2::timestamptz IS NULL OR al.created_at >= $2)
                  AND ($3::timestamptz IS NULL OR al.created_at <= $3)
                  AND ($4::text IS NULL OR al.action = $4)
                  AND ($5::text IS NULL OR al.entity_kind = $5)
                  AND ($6::uuid IS NULL OR al.actor_user_id = $6)
                ORDER BY al.created_at DESC, al.id DESC
                LIMIT $7 OFFSET $8
            )
            SELECT
                f.id,
                f.created_at,
                f.action,
                f.entity_kind,
                f.entity_id,
                f.metadata,
                f.actor_user_id,
                f.actor_display_name,
                f.actor_email,
                f.entity_person_id,
                f.total_count,
                CASE
                    WHEN p.id IS NULL THEN NULL
                    ELSE p.given_name || ' ' || p.family_name
                END AS entity_person_name
            FROM filtered f
            LEFT JOIN persons p ON p.id = f.entity_person_id
            ORDER BY f.created_at DESC, f.id DESC
            ",
        )
        .bind(filter.family_id.into_uuid())
        .bind(filter.from)
        .bind(filter.to)
        .bind(filter.action.as_deref())
        .bind(filter.entity_kind.as_deref())
        .bind(filter.actor_user_id.map(UserId::into_uuid))
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AuditRepoError::Db(e.to_string()))?;

        let total: i64 = rows.first().map_or(0, |r| r.get::<i64, _>("total_count"));

        let out: Vec<AuditRow> = rows
            .into_iter()
            .map(|r| AuditRow {
                id: r.get::<Uuid, _>("id"),
                created_at: r.get("created_at"),
                action: r.get::<String, _>("action"),
                entity_kind: r.get::<String, _>("entity_kind"),
                entity_id: r.get::<Option<Uuid>, _>("entity_id"),
                metadata: r.get::<Value, _>("metadata"),
                actor_user_id: r.get::<Option<Uuid>, _>("actor_user_id").map(UserId::from_uuid),
                actor_display_name: r.get::<Option<String>, _>("actor_display_name"),
                actor_email: r.get::<Option<String>, _>("actor_email"),
                entity_person_id: r.get::<Option<Uuid>, _>("entity_person_id"),
                entity_person_name: r.get::<Option<String>, _>("entity_person_name"),
            })
            .collect();

        Ok((out, AuditPageMeta { total }))
    }
}
