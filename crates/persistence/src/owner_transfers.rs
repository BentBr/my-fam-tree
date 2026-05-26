//! Postgres-backed [`OwnerTransferRepo`].
//!
//! All four mutating methods are single-statement SQL. The partial unique
//! index `family_owner_transfers_active_idx` enforces "at most one pending
//! transfer per family" at the DB layer — `begin` maps Postgres error code
//! `23505` (`unique_violation`) to [`OwnerTransferRepoError::AlreadyPending`]
//! so the API layer can surface it as a `409 Conflict`.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use my_family_domain::{
    FamilyId, OwnerTransfer, OwnerTransferRepo, OwnerTransferRepoError, TransferSide, UserId,
};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct PgOwnerTransferRepo {
    pool: PgPool,
}

impl PgOwnerTransferRepo {
    #[must_use]
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl OwnerTransferRepo for PgOwnerTransferRepo {
    async fn begin(
        &self,
        family_id: FamilyId,
        from_user_id: UserId,
        to_user_id: UserId,
        from_token_hash: &[u8],
        to_token_hash: &[u8],
        expires_at: DateTime<Utc>,
    ) -> Result<Uuid, OwnerTransferRepoError> {
        let row = sqlx::query!(
            r#"INSERT INTO family_owner_transfers
               (family_id, from_user_id, to_user_id, from_token_hash, to_token_hash, expires_at)
               VALUES ($1, $2, $3, $4, $5, $6)
               RETURNING id"#,
            family_id.into_uuid(),
            from_user_id.into_uuid(),
            to_user_id.into_uuid(),
            from_token_hash,
            to_token_hash,
            expires_at,
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            // unique_violation on the partial index = a pending transfer already exists.
            if let Some(db_err) = e.as_database_error()
                && db_err.code().as_deref() == Some("23505")
            {
                return OwnerTransferRepoError::AlreadyPending;
            }
            OwnerTransferRepoError::Db(e.to_string())
        })?;
        Ok(row.id)
    }

    async fn confirm(
        &self,
        token_hash: &[u8],
        now: DateTime<Utc>,
    ) -> Result<(OwnerTransfer, TransferSide), OwnerTransferRepoError> {
        // First find which side this token belongs to.
        let row = sqlx::query!(
            r#"SELECT id, family_id, from_user_id, to_user_id,
                      from_token_hash, to_token_hash,
                      from_confirmed_at, to_confirmed_at,
                      expires_at, completed_at, cancelled_at
               FROM family_owner_transfers
               WHERE (from_token_hash = $1 OR to_token_hash = $1)
                 AND completed_at IS NULL AND cancelled_at IS NULL
               LIMIT 1"#,
            token_hash,
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| OwnerTransferRepoError::Db(e.to_string()))?
        .ok_or(OwnerTransferRepoError::NotFound)?;

        if row.expires_at < now {
            return Err(OwnerTransferRepoError::Expired);
        }

        let side =
            if row.from_token_hash == token_hash { TransferSide::From } else { TransferSide::To };

        // Mark the matching side as confirmed. Each `query!` macro
        // generates its own anonymous `Record` struct, so we convert to
        // the public `OwnerTransfer` inside each match arm to keep the
        // arms' types unified.
        let updated = match side {
            TransferSide::From => {
                let r = sqlx::query!(
                    r#"UPDATE family_owner_transfers
                       SET from_confirmed_at = COALESCE(from_confirmed_at, $2)
                       WHERE id = $1
                       RETURNING id, family_id, from_user_id, to_user_id,
                                 from_confirmed_at, to_confirmed_at,
                                 expires_at, completed_at, cancelled_at"#,
                    row.id,
                    now,
                )
                .fetch_one(&self.pool)
                .await
                .map_err(|e| OwnerTransferRepoError::Db(e.to_string()))?;
                OwnerTransfer {
                    id: r.id,
                    family_id: FamilyId::from_uuid(r.family_id),
                    from_user_id: UserId::from_uuid(r.from_user_id),
                    to_user_id: UserId::from_uuid(r.to_user_id),
                    from_confirmed_at: r.from_confirmed_at,
                    to_confirmed_at: r.to_confirmed_at,
                    expires_at: r.expires_at,
                    completed_at: r.completed_at,
                    cancelled_at: r.cancelled_at,
                }
            }
            TransferSide::To => {
                let r = sqlx::query!(
                    r#"UPDATE family_owner_transfers
                       SET to_confirmed_at = COALESCE(to_confirmed_at, $2)
                       WHERE id = $1
                       RETURNING id, family_id, from_user_id, to_user_id,
                                 from_confirmed_at, to_confirmed_at,
                                 expires_at, completed_at, cancelled_at"#,
                    row.id,
                    now,
                )
                .fetch_one(&self.pool)
                .await
                .map_err(|e| OwnerTransferRepoError::Db(e.to_string()))?;
                OwnerTransfer {
                    id: r.id,
                    family_id: FamilyId::from_uuid(r.family_id),
                    from_user_id: UserId::from_uuid(r.from_user_id),
                    to_user_id: UserId::from_uuid(r.to_user_id),
                    from_confirmed_at: r.from_confirmed_at,
                    to_confirmed_at: r.to_confirmed_at,
                    expires_at: r.expires_at,
                    completed_at: r.completed_at,
                    cancelled_at: r.cancelled_at,
                }
            }
        };

        Ok((updated, side))
    }

    async fn complete(&self, id: Uuid, now: DateTime<Utc>) -> Result<(), OwnerTransferRepoError> {
        sqlx::query!(
            "UPDATE family_owner_transfers SET completed_at = $2
             WHERE id = $1 AND completed_at IS NULL AND cancelled_at IS NULL",
            id,
            now,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| OwnerTransferRepoError::Db(e.to_string()))?;
        Ok(())
    }

    async fn cancel(
        &self,
        family_id: FamilyId,
        now: DateTime<Utc>,
    ) -> Result<(), OwnerTransferRepoError> {
        sqlx::query!(
            "UPDATE family_owner_transfers SET cancelled_at = $2
             WHERE family_id = $1 AND completed_at IS NULL AND cancelled_at IS NULL",
            family_id.into_uuid(),
            now,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| OwnerTransferRepoError::Db(e.to_string()))?;
        Ok(())
    }

    async fn find_active(
        &self,
        family_id: FamilyId,
    ) -> Result<Option<OwnerTransfer>, OwnerTransferRepoError> {
        let row = sqlx::query!(
            r#"SELECT id, family_id, from_user_id, to_user_id,
                      from_confirmed_at, to_confirmed_at,
                      expires_at, completed_at, cancelled_at
               FROM family_owner_transfers
               WHERE family_id = $1 AND completed_at IS NULL AND cancelled_at IS NULL
               LIMIT 1"#,
            family_id.into_uuid(),
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| OwnerTransferRepoError::Db(e.to_string()))?;

        Ok(row.map(|r| OwnerTransfer {
            id: r.id,
            family_id: FamilyId::from_uuid(r.family_id),
            from_user_id: UserId::from_uuid(r.from_user_id),
            to_user_id: UserId::from_uuid(r.to_user_id),
            from_confirmed_at: r.from_confirmed_at,
            to_confirmed_at: r.to_confirmed_at,
            expires_at: r.expires_at,
            completed_at: r.completed_at,
            cancelled_at: r.cancelled_at,
        }))
    }
}
