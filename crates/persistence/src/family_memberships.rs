//! Postgres-backed [`FamilyMembershipRepo`] implementation.

use async_trait::async_trait;
use my_family_domain::{
    FamilyId, FamilyMembershipRepo, Membership, MembershipRepoError, MembershipWithFamilyName,
    Role, UserId,
};
use sqlx::PgPool;

#[derive(Clone, Debug)]
pub struct PgFamilyMembershipRepo {
    pool: PgPool,
}

impl PgFamilyMembershipRepo {
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
        let rows = sqlx::query!(
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
        Ok(rows
            .into_iter()
            .map(|r| MembershipWithFamilyName {
                family_id: FamilyId::from_uuid(r.family_id),
                family_name: r.family_name,
                role: role_from_db(&r.role),
            })
            .collect())
    }

    async fn find(
        &self,
        family_id: FamilyId,
        user_id: UserId,
    ) -> Result<Option<Membership>, MembershipRepoError> {
        let row = sqlx::query!(
            r#"SELECT family_id, user_id, role::text AS "role!", joined_at
                 FROM family_memberships WHERE family_id = $1 AND user_id = $2"#,
            family_id.into_uuid(),
            user_id.into_uuid()
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| MembershipRepoError::Db(e.to_string()))?;
        Ok(row.map(|r| Membership {
            family_id: FamilyId::from_uuid(r.family_id),
            user_id: UserId::from_uuid(r.user_id),
            role: role_from_db(&r.role),
            joined_at: r.joined_at,
        }))
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
}
