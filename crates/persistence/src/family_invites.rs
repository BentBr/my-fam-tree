//! Postgres-backed [`FamilyInviteRepo`] implementation.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use my_family_domain::{FamilyId, FamilyInviteRepo, Invite, InviteRepoError, Role, UserId};
use sqlx::PgPool;
use uuid::Uuid;

const fn role_db(r: Role) -> &'static str {
    match r {
        Role::User => "user",
        Role::Admin => "admin",
        Role::Owner => "owner",
    }
}

fn role_from_db(s: &str) -> Role {
    match s {
        "owner" => Role::Owner,
        "admin" => Role::Admin,
        _ => Role::User,
    }
}

#[derive(Clone, Debug)]
pub struct PgFamilyInviteRepo {
    pool: PgPool,
}

impl PgFamilyInviteRepo {
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl FamilyInviteRepo for PgFamilyInviteRepo {
    async fn create(
        &self,
        family_id: FamilyId,
        email: &str,
        invited_role: Role,
        invited_by: UserId,
        token_hash: &[u8],
        expires_at: DateTime<Utc>,
    ) -> Result<Uuid, InviteRepoError> {
        let row = sqlx::query!(
            r#"INSERT INTO family_invites
               (family_id, email, invited_role, invited_by, token_hash, expires_at)
               VALUES ($1, $2, ($3::text)::family_role, $4, $5, $6)
               RETURNING id"#,
            family_id.into_uuid(),
            email,
            role_db(invited_role),
            invited_by.into_uuid(),
            token_hash,
            expires_at,
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| InviteRepoError::Db(e.to_string()))?;
        Ok(row.id)
    }

    async fn accept(
        &self,
        token_hash: &[u8],
        now: DateTime<Utc>,
    ) -> Result<Invite, InviteRepoError> {
        let row = sqlx::query!(
            r#"UPDATE family_invites SET accepted_at = now()
                WHERE token_hash = $1 AND accepted_at IS NULL
                RETURNING id, family_id, email::text AS "email!", invited_role::text AS "role!",
                          invited_by, expires_at, accepted_at"#,
            token_hash
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| InviteRepoError::Db(e.to_string()))?
        .ok_or(InviteRepoError::NotFoundOrAccepted)?;

        if row.expires_at < now {
            return Err(InviteRepoError::Expired);
        }
        Ok(Invite {
            id: row.id,
            family_id: FamilyId::from_uuid(row.family_id),
            email: row.email,
            invited_role: role_from_db(&row.role),
            invited_by: UserId::from_uuid(row.invited_by),
            expires_at: row.expires_at,
            accepted_at: row.accepted_at,
        })
    }

    async fn list_pending_for_family(
        &self,
        family_id: FamilyId,
    ) -> Result<Vec<Invite>, InviteRepoError> {
        let rows = sqlx::query!(
            r#"SELECT id, family_id, email::text AS "email!", invited_role::text AS "role!",
                      invited_by, expires_at, accepted_at
                 FROM family_invites
                WHERE family_id = $1 AND accepted_at IS NULL AND expires_at > now()
                ORDER BY created_at DESC"#,
            family_id.into_uuid()
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| InviteRepoError::Db(e.to_string()))?;
        Ok(rows
            .into_iter()
            .map(|r| Invite {
                id: r.id,
                family_id: FamilyId::from_uuid(r.family_id),
                email: r.email,
                invited_role: role_from_db(&r.role),
                invited_by: UserId::from_uuid(r.invited_by),
                expires_at: r.expires_at,
                accepted_at: r.accepted_at,
            })
            .collect())
    }

    async fn cancel(&self, id: Uuid, family_id: FamilyId) -> Result<(), InviteRepoError> {
        let res = sqlx::query!(
            r#"DELETE FROM family_invites
                WHERE id = $1 AND family_id = $2 AND accepted_at IS NULL"#,
            id,
            family_id.into_uuid()
        )
        .execute(&self.pool)
        .await
        .map_err(|e| InviteRepoError::Db(e.to_string()))?;
        if res.rows_affected() == 0 {
            return Err(InviteRepoError::NotFoundOrAccepted);
        }
        Ok(())
    }
}
