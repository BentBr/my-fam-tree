//! Postgres-backed [`MagicLinkRepo`] implementation.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use my_family_domain::{
    MagicLinkPurpose, MagicLinkRecord, MagicLinkRepo, MagicLinkRepoError, UserId,
};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct PgMagicLinkRepo {
    pool: PgPool,
}

impl PgMagicLinkRepo {
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

fn purpose_from_db(s: &str) -> MagicLinkPurpose {
    match s {
        "login" => MagicLinkPurpose::Login,
        "invite_accept" => MagicLinkPurpose::InviteAccept,
        _ => MagicLinkPurpose::EmailChange,
    }
}

#[async_trait]
impl MagicLinkRepo for PgMagicLinkRepo {
    async fn create(
        &self,
        user_id: Option<UserId>,
        email: &str,
        token_hash: &[u8],
        purpose: MagicLinkPurpose,
        expires_at: DateTime<Utc>,
    ) -> Result<Uuid, MagicLinkRepoError> {
        let user_uuid = user_id.map(UserId::into_uuid);
        let row = sqlx::query!(
            r#"INSERT INTO magic_link_tokens (user_id, email, token_hash, purpose, expires_at)
               VALUES ($1, $2, $3, ($4::text)::magic_link_purpose, $5)
               RETURNING id"#,
            user_uuid,
            email,
            token_hash,
            purpose.as_db(),
            expires_at
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| MagicLinkRepoError::Db(e.to_string()))?;
        Ok(row.id)
    }

    async fn consume(&self, token_hash: &[u8]) -> Result<MagicLinkRecord, MagicLinkRepoError> {
        let row = sqlx::query!(
            r#"UPDATE magic_link_tokens SET consumed_at = now()
                WHERE token_hash = $1 AND consumed_at IS NULL
                RETURNING id, user_id, email::text AS "email!", purpose::text AS "purpose!",
                          expires_at, consumed_at"#,
            token_hash
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| MagicLinkRepoError::Db(e.to_string()))?
        .ok_or(MagicLinkRepoError::NotFoundOrConsumed)?;

        if row.expires_at < Utc::now() {
            return Err(MagicLinkRepoError::Expired);
        }
        Ok(MagicLinkRecord {
            id: row.id,
            user_id: row.user_id.map(UserId::from_uuid),
            email: row.email,
            purpose: purpose_from_db(&row.purpose),
            expires_at: row.expires_at,
            consumed_at: row.consumed_at,
        })
    }
}
