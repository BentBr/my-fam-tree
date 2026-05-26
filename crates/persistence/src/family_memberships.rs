//! Postgres-backed [`FamilyMembershipRepo`] implementation.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use my_family_domain::{
    FamilyId, FamilyMembershipRepo, MemberWithUser, Membership, MembershipRepoError,
    MembershipWithFamilyName, Role, UserId,
};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct PgFamilyMembershipRepo {
    pool: PgPool,
}

impl PgFamilyMembershipRepo {
    #[must_use]
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

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

/// Mirror of the columns selected by `family_memberships` queries that return [`Membership`].
/// Centralized so the row → `Membership` conversion lives in exactly one place.
#[derive(sqlx::FromRow)]
struct MembershipRow {
    family_id: Uuid,
    user_id: Uuid,
    #[sqlx(rename = "role!")]
    role: String,
    joined_at: DateTime<Utc>,
}

impl From<MembershipRow> for Membership {
    fn from(r: MembershipRow) -> Self {
        Self {
            family_id: FamilyId::from_uuid(r.family_id),
            user_id: UserId::from_uuid(r.user_id),
            role: role_from_db(&r.role),
            joined_at: r.joined_at,
        }
    }
}

/// Mirror of the JOIN row returned by `list_for_user`.
#[derive(sqlx::FromRow)]
struct MembershipWithFamilyNameRow {
    family_id: Uuid,
    family_name: String,
    #[sqlx(rename = "role!")]
    role: String,
}

impl From<MembershipWithFamilyNameRow> for MembershipWithFamilyName {
    fn from(r: MembershipWithFamilyNameRow) -> Self {
        Self {
            family_id: FamilyId::from_uuid(r.family_id),
            family_name: r.family_name,
            role: role_from_db(&r.role),
        }
    }
}

#[async_trait]
impl FamilyMembershipRepo for PgFamilyMembershipRepo {
    async fn insert(
        &self,
        family_id: FamilyId,
        user_id: UserId,
        role: Role,
    ) -> Result<(), MembershipRepoError> {
        let res = sqlx::query!(
            r#"INSERT INTO family_memberships (family_id, user_id, role)
               VALUES ($1, $2, ($3::text)::family_role)"#,
            family_id.into_uuid(),
            user_id.into_uuid(),
            role_db(role)
        )
        .execute(&self.pool)
        .await;

        if let Err(sqlx::Error::Database(ref db_err)) = res
            && db_err.constraint() == Some("family_memberships_one_owner")
        {
            return Err(MembershipRepoError::OwnerExists);
        }
        res.map_err(|e| MembershipRepoError::Db(e.to_string()))?;
        Ok(())
    }

    async fn list_for_user(
        &self,
        user_id: UserId,
    ) -> Result<Vec<MembershipWithFamilyName>, MembershipRepoError> {
        let rows = sqlx::query_as!(
            MembershipWithFamilyNameRow,
            r#"SELECT fm.family_id, f.name AS family_name, fm.role::text AS "role!"
                 FROM family_memberships fm
                 JOIN families f ON f.id = fm.family_id
                WHERE fm.user_id = $1
                ORDER BY f.name"#,
            user_id.into_uuid()
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| MembershipRepoError::Db(e.to_string()))?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn find(
        &self,
        family_id: FamilyId,
        user_id: UserId,
    ) -> Result<Option<Membership>, MembershipRepoError> {
        let row = sqlx::query_as!(
            MembershipRow,
            r#"SELECT family_id, user_id, role::text AS "role!", joined_at
                 FROM family_memberships WHERE family_id = $1 AND user_id = $2"#,
            family_id.into_uuid(),
            user_id.into_uuid()
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| MembershipRepoError::Db(e.to_string()))?;
        Ok(row.map(Into::into))
    }

    async fn set_role(
        &self,
        family_id: FamilyId,
        user_id: UserId,
        role: Role,
    ) -> Result<(), MembershipRepoError> {
        let res = sqlx::query!(
            r#"UPDATE family_memberships SET role = ($3::text)::family_role
                WHERE family_id = $1 AND user_id = $2"#,
            family_id.into_uuid(),
            user_id.into_uuid(),
            role_db(role)
        )
        .execute(&self.pool)
        .await;
        if let Err(sqlx::Error::Database(ref db_err)) = res
            && db_err.constraint() == Some("family_memberships_one_owner")
        {
            return Err(MembershipRepoError::OwnerExists);
        }
        let res = res.map_err(|e| MembershipRepoError::Db(e.to_string()))?;
        if res.rows_affected() == 0 {
            return Err(MembershipRepoError::NotMember);
        }
        Ok(())
    }

    async fn remove(
        &self,
        family_id: FamilyId,
        user_id: UserId,
    ) -> Result<(), MembershipRepoError> {
        let res = sqlx::query!(
            r#"DELETE FROM family_memberships WHERE family_id = $1 AND user_id = $2"#,
            family_id.into_uuid(),
            user_id.into_uuid()
        )
        .execute(&self.pool)
        .await
        .map_err(|e| MembershipRepoError::Db(e.to_string()))?;
        if res.rows_affected() == 0 {
            return Err(MembershipRepoError::NotMember);
        }
        Ok(())
    }

    async fn list_with_users(
        &self,
        family_id: FamilyId,
    ) -> Result<Vec<MemberWithUser>, MembershipRepoError> {
        // Sort order is owner → admin → user via the CASE; ties (e.g.
        // multiple admins) break on display name so the FE table is
        // stable across reloads. `users.email` is a CITEXT domain — the
        // explicit `::text` cast keeps SQLx's prepare step happy and
        // produces a plain `String` instead of an opaque type.
        let rows = sqlx::query!(
            r#"
            SELECT
                fm.user_id,
                fm.role::text  AS "role!",
                fm.joined_at,
                u.email::text  AS "email!",
                u.display_name AS "display_name!"
            FROM family_memberships fm
            JOIN users u ON u.id = fm.user_id
            WHERE fm.family_id = $1
            ORDER BY
                CASE fm.role
                    WHEN 'owner' THEN 0
                    WHEN 'admin' THEN 1
                    ELSE 2
                END,
                u.display_name
            "#,
            family_id.into_uuid(),
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| MembershipRepoError::Db(e.to_string()))?;

        Ok(rows
            .into_iter()
            .map(|r| MemberWithUser {
                user_id: UserId::from_uuid(r.user_id),
                email: r.email,
                display_name: r.display_name,
                role: role_from_db(&r.role),
                joined_at: r.joined_at,
            })
            .collect())
    }
}
