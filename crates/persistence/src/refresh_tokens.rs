//! Postgres-backed [`RefreshTokenRepo`] implementation.

use std::net::IpAddr;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use my_family_domain::{RefreshRepoError, RefreshTokenRecord, RefreshTokenRepo, UserId};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct PgRefreshTokenRepo {
    pool: PgPool,
}

impl PgRefreshTokenRepo {
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

/// Mirror of the columns selected by `refresh_tokens` queries that return [`RefreshTokenRecord`].
/// Centralized so the row → `RefreshTokenRecord` conversion lives in exactly one place.
#[derive(sqlx::FromRow)]
struct RefreshTokenRow {
    id: Uuid,
    user_id: Uuid,
    created_at: DateTime<Utc>,
    last_used_at: DateTime<Utc>,
    expires_at: DateTime<Utc>,
    absolute_expires_at: DateTime<Utc>,
    revoked_at: Option<DateTime<Utc>>,
    device_label: Option<String>,
}

impl From<RefreshTokenRow> for RefreshTokenRecord {
    fn from(r: RefreshTokenRow) -> Self {
        Self {
            id: r.id,
            user_id: UserId::from_uuid(r.user_id),
            created_at: r.created_at,
            last_used_at: r.last_used_at,
            expires_at: r.expires_at,
            absolute_expires_at: r.absolute_expires_at,
            revoked_at: r.revoked_at,
            device_label: r.device_label,
        }
    }
}

#[async_trait]
impl RefreshTokenRepo for PgRefreshTokenRepo {
    async fn create(
        &self,
        user_id: UserId,
        token_hash: &[u8],
        device_label: Option<&str>,
        user_agent: Option<&str>,
        ip: Option<IpAddr>,
        rolling_expires_at: DateTime<Utc>,
        absolute_expires_at: DateTime<Utc>,
    ) -> Result<Uuid, RefreshRepoError> {
        let ip_net = ip.map(sqlx::types::ipnetwork::IpNetwork::from);
        let row = sqlx::query!(
            r#"INSERT INTO refresh_tokens
               (user_id, token_hash, device_label, user_agent, ip, expires_at, absolute_expires_at)
               VALUES ($1, $2, $3, $4, $5, $6, $7)
               RETURNING id"#,
            user_id.into_uuid(),
            token_hash,
            device_label,
            user_agent,
            ip_net,
            rolling_expires_at,
            absolute_expires_at,
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| RefreshRepoError::Db(e.to_string()))?;
        Ok(row.id)
    }

    async fn find_active_by_hash(
        &self,
        token_hash: &[u8],
    ) -> Result<Option<RefreshTokenRecord>, RefreshRepoError> {
        let row = sqlx::query_as!(
            RefreshTokenRow,
            r#"SELECT id, user_id, created_at, last_used_at, expires_at, absolute_expires_at,
                      revoked_at, device_label
                 FROM refresh_tokens WHERE token_hash = $1 AND revoked_at IS NULL"#,
            token_hash
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| RefreshRepoError::Db(e.to_string()))?;
        Ok(row.map(Into::into))
    }

    async fn rotate(
        &self,
        old_hash: &[u8],
        new_hash: &[u8],
        new_rolling_expires_at: DateTime<Utc>,
        device_label: Option<&str>,
        user_agent: Option<&str>,
        ip: Option<IpAddr>,
    ) -> Result<(), RefreshRepoError> {
        let ip_net = ip.map(sqlx::types::ipnetwork::IpNetwork::from);
        let mut tx = self.pool.begin().await.map_err(|e| RefreshRepoError::Db(e.to_string()))?;
        let row = sqlx::query!(
            r#"UPDATE refresh_tokens SET revoked_at = now()
                WHERE token_hash = $1 AND revoked_at IS NULL
                RETURNING user_id, absolute_expires_at"#,
            old_hash
        )
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| RefreshRepoError::Db(e.to_string()))?
        .ok_or(RefreshRepoError::NotFound)?;

        sqlx::query!(
            r#"INSERT INTO refresh_tokens
               (user_id, token_hash, device_label, user_agent, ip, expires_at, absolute_expires_at)
               VALUES ($1, $2, $3, $4, $5, $6, $7)"#,
            row.user_id,
            new_hash,
            device_label,
            user_agent,
            ip_net,
            new_rolling_expires_at,
            row.absolute_expires_at,
        )
        .execute(&mut *tx)
        .await
        .map_err(|e| RefreshRepoError::Db(e.to_string()))?;

        tx.commit().await.map_err(|e| RefreshRepoError::Db(e.to_string()))?;
        Ok(())
    }

    async fn revoke_by_hash(&self, token_hash: &[u8]) -> Result<(), RefreshRepoError> {
        sqlx::query!(
            r#"UPDATE refresh_tokens SET revoked_at = now()
                WHERE token_hash = $1 AND revoked_at IS NULL"#,
            token_hash
        )
        .execute(&self.pool)
        .await
        .map_err(|e| RefreshRepoError::Db(e.to_string()))?;
        Ok(())
    }

    async fn revoke_all_for_user(&self, user_id: UserId) -> Result<u64, RefreshRepoError> {
        let res = sqlx::query!(
            r#"UPDATE refresh_tokens SET revoked_at = now()
                WHERE user_id = $1 AND revoked_at IS NULL"#,
            user_id.into_uuid()
        )
        .execute(&self.pool)
        .await
        .map_err(|e| RefreshRepoError::Db(e.to_string()))?;
        Ok(res.rows_affected())
    }

    async fn list_for_user(
        &self,
        user_id: UserId,
    ) -> Result<Vec<RefreshTokenRecord>, RefreshRepoError> {
        let rows = sqlx::query_as!(
            RefreshTokenRow,
            r#"SELECT id, user_id, created_at, last_used_at, expires_at, absolute_expires_at,
                      revoked_at, device_label
                 FROM refresh_tokens WHERE user_id = $1 AND revoked_at IS NULL
                 ORDER BY last_used_at DESC"#,
            user_id.into_uuid()
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| RefreshRepoError::Db(e.to_string()))?;
        Ok(rows.into_iter().map(Into::into).collect())
    }
}
