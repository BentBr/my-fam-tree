//! Postgres-backed [`FamilyMembershipRepo`] implementation.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use my_fam_tree_domain::{
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
    created_at: DateTime<Utc>,
}

impl From<MembershipWithFamilyNameRow> for MembershipWithFamilyName {
    fn from(r: MembershipWithFamilyNameRow) -> Self {
        Self {
            family_id: FamilyId::from_uuid(r.family_id),
            family_name: r.family_name,
            role: role_from_db(&r.role),
            created_at: r.created_at,
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
        // `ON CONFLICT (family_id, user_id) DO NOTHING` makes the insert
        // idempotent on the membership row — a no-op when the user is
        // already a member of this family. Two scenarios this covers
        // both come down to "the invite-accept handler ran twice for
        // the same (family, user) pair":
        //   1. the same invite link is re-clicked;
        //   2. a follow-up person-targeted invite arrives for a user
        //      who already has a family-level membership.
        // In both cases the post-membership side-effects in
        // `routes::invites::accept` (audit row + `set_linked_user_id`
        // on `invite.person_id`) still need to run, so the membership
        // insert must succeed-quietly rather than reject.
        //
        // Deliberately NOT `DO UPDATE SET role = EXCLUDED.role`: a
        // re-accept MUST NOT silently change the role. Role changes
        // route through the dedicated members endpoint, which audits
        // the change and enforces owner-cannot-self-demote. Letting a
        // re-accept upgrade `role` would be a self-promotion vector
        // for anyone who can craft an invite for themselves.
        //
        // The owner-existence partial-unique constraint
        // (`family_memberships_one_owner`) is on a different column
        // set than the PK and is NOT silenced by this `ON CONFLICT`.
        // Attempts to insert a SECOND owner for the same family still
        // error out, and the explicit match below maps that to
        // `OwnerExists`.
        let res = sqlx::query!(
            r#"INSERT INTO family_memberships (family_id, user_id, role)
               VALUES ($1, $2, ($3::text)::family_role)
               ON CONFLICT (family_id, user_id) DO NOTHING"#,
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
            r#"SELECT fm.family_id, f.name AS family_name, fm.role::text AS "role!", f.created_at
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
        // LEFT JOIN to persons so we surface the user's "name in this
        // family" when their account `display_name` is empty: most
        // members never set a display name, but they DO have a person
        // row (created by an admin during seeding / invite) that
        // everyone in the family already knows them by. The COALESCE
        // turns ' ' (which `given || ' ' || family` would yield for a
        // row with empty both halves) into NULL so the FE can clean-
        // fall through to email. The sort key still uses display_name
        // first then linked-person name so the rendered order matches
        // the rendered label.
        let rows = sqlx::query!(
            r#"
            SELECT
                fm.user_id,
                fm.role::text  AS "role!",
                fm.joined_at,
                u.email::text  AS "email!",
                u.display_name AS "display_name!",
                NULLIF(TRIM(BOTH FROM COALESCE(p.given_name, '') || ' ' || COALESCE(p.family_name, '')), '')
                    AS "linked_person_name?"
            FROM family_memberships fm
            JOIN users u ON u.id = fm.user_id
            LEFT JOIN persons p
                   ON p.linked_user_id = fm.user_id AND p.family_id = fm.family_id
            WHERE fm.family_id = $1
            ORDER BY
                CASE fm.role
                    WHEN 'owner' THEN 0
                    WHEN 'admin' THEN 1
                    ELSE 2
                END,
                NULLIF(u.display_name, ''),
                NULLIF(TRIM(BOTH FROM COALESCE(p.given_name, '') || ' ' || COALESCE(p.family_name, '')), ''),
                u.email
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
                linked_person_name: r.linked_person_name,
            })
            .collect())
    }

    async fn count_in_family(&self, family_id: FamilyId) -> Result<u64, MembershipRepoError> {
        let row = sqlx::query!(
            r#"SELECT COUNT(*) AS "count!: i64" FROM family_memberships WHERE family_id = $1"#,
            family_id.into_uuid(),
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| MembershipRepoError::Db(e.to_string()))?;
        Ok(u64::try_from(row.count).unwrap_or(0))
    }
}
