use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::{FamilyId, Role, UserId};

#[derive(Debug, Clone)]
pub struct Membership {
    pub family_id: FamilyId,
    pub user_id: UserId,
    pub role: Role,
    pub joined_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct MembershipWithFamilyName {
    pub family_id: FamilyId,
    pub family_name: String,
    pub role: Role,
    /// When the family was created (`families.created_at`). Display-only —
    /// lets the FE family switcher disambiguate same-named families. NOT
    /// carried in the JWT family claim (auth-critical, kept lean).
    pub created_at: DateTime<Utc>,
}

/// Member row enriched with the joined user's display fields.
///
/// Used by the admin Members page so the FE can render name + email
/// next to the role chip without a second round-trip. The bare
/// [`Membership`] value still drives auth — no user fields leak into
/// JWT-shaped paths.
#[derive(Debug, Clone)]
pub struct MemberWithUser {
    pub user_id: UserId,
    pub email: String,
    pub display_name: String,
    pub role: Role,
    pub joined_at: DateTime<Utc>,
    /// Name of the `persons` row (in this family) that this user has
    /// been linked to via `linked_user_id`, if any. The admin Members
    /// view falls back to this when the user's account `display_name`
    /// is empty — most members never bother to set a display name and
    /// the linked-person name is what the family already knows them
    /// as. `None` when the user has no linked person in this family
    /// (the FE then falls back to the raw email).
    pub linked_person_name: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum MembershipRepoError {
    #[error("database: {0}")]
    Db(String),
    #[error("not a member")]
    NotMember,
    #[error("family already has an owner")]
    OwnerExists,
}

#[async_trait]
pub trait FamilyMembershipRepo: Send + Sync {
    async fn insert(
        &self,
        family_id: FamilyId,
        user_id: UserId,
        role: Role,
    ) -> Result<(), MembershipRepoError>;
    async fn list_for_user(
        &self,
        user_id: UserId,
    ) -> Result<Vec<MembershipWithFamilyName>, MembershipRepoError>;
    async fn find(
        &self,
        family_id: FamilyId,
        user_id: UserId,
    ) -> Result<Option<Membership>, MembershipRepoError>;
    async fn set_role(
        &self,
        family_id: FamilyId,
        user_id: UserId,
        role: Role,
    ) -> Result<(), MembershipRepoError>;
    async fn remove(&self, family_id: FamilyId, user_id: UserId)
    -> Result<(), MembershipRepoError>;
    /// List every member of `family_id` joined to `users` for the
    /// display name and email, ordered owner → admin → user, then by
    /// display name. Used by the admin `/admin/members` view.
    async fn list_with_users(
        &self,
        family_id: FamilyId,
    ) -> Result<Vec<MemberWithUser>, MembershipRepoError>;

    /// Total number of members in `family_id`. Used by the admin
    /// family-overview endpoint to surface a single headline number
    /// without pulling the full member list across the wire.
    async fn count_in_family(&self, family_id: FamilyId) -> Result<u64, MembershipRepoError>;
}
