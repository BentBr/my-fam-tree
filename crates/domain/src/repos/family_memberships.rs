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
    async fn remove(
        &self,
        family_id: FamilyId,
        user_id: UserId,
    ) -> Result<(), MembershipRepoError>;
}
