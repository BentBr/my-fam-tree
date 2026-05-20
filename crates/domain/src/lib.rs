//! Core domain types and repository traits. No I/O.

pub mod capabilities;
pub mod ids;
pub mod repos;
pub mod role;

pub use capabilities::{Capability, capabilities_of, has};
pub use ids::{FamilyId, FamilyMembershipId, PersonId, UserId};
pub use repos::{
    families::{Family, FamilyRepo, FamilyRepoError},
    family_invites::{FamilyInviteRepo, Invite, InviteRepoError},
    family_memberships::{
        FamilyMembershipRepo, Membership, MembershipRepoError, MembershipWithFamilyName,
    },
    magic_link_tokens::{MagicLinkPurpose, MagicLinkRecord, MagicLinkRepo, MagicLinkRepoError},
    refresh_tokens::{RefreshRepoError, RefreshTokenRecord, RefreshTokenRepo},
    users::{Locale, User, UserRepo, UserRepoError},
};
pub use role::Role;
