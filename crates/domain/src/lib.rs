//! Core domain types and repository traits. No I/O.

pub mod capabilities;
pub mod ids;
pub mod relationships;
pub mod repos;
pub mod role;
pub mod upcoming;

pub use capabilities::{Capability, capabilities_of, has};
pub use ids::{FamilyId, FamilyMembershipId, PersonId, UserId};
pub use relationships::{canonicalize_pair, would_create_cycle};
pub use repos::audit_log::{
    AuditEntry, AuditFilter, AuditLogRepo, AuditPageMeta, AuditRepoError, AuditRow,
};
pub use repos::families::{Family, FamilyRepo, FamilyRepoError};
pub use repos::family_invites::{FamilyInviteRepo, Invite, InviteRepoError};
pub use repos::family_memberships::{
    FamilyMembershipRepo, MemberWithUser, Membership, MembershipRepoError, MembershipWithFamilyName,
};
pub use repos::health::{HealthRepo, HealthRepoError};
pub use repos::janitor::{JanitorRepo, JanitorRepoError, JanitorSweepReport};
pub use repos::magic_link_tokens::{
    MagicLinkPurpose, MagicLinkRecord, MagicLinkRepo, MagicLinkRepoError,
};
pub use repos::owner_transfers::{
    OwnerTransfer, OwnerTransferRepo, OwnerTransferRepoError, TransferSide,
};
pub use repos::parent_links::{ParentKind, ParentLink, ParentLinkRepo, ParentLinkRepoError};
pub use repos::partnerships::{
    Partnership, PartnershipDraft, PartnershipEndReason, PartnershipKind, PartnershipRepo,
    PartnershipRepoError,
};
pub use repos::person_contacts::{
    Contact, ContactDraft, ContactKind, ContactRepoError, ContactVisibility, PersonContactRepo,
};
pub use repos::person_favourites::{PersonFavouriteRepo, PersonFavouriteRepoError};
pub use repos::persons::{Person, PersonDraft, PersonRepo, PersonRepoError};
pub use repos::refresh_tokens::{RefreshRepoError, RefreshTokenRecord, RefreshTokenRepo};
pub use repos::reminder_digests::{
    DigestRepoError, DigestStatus, ReminderDigest, ReminderDigestRepo,
};
pub use repos::reminder_prefs::{
    ReminderPreferences, ReminderPreferencesRepo, ReminderPrefsRepoError,
};
pub use repos::users::{Locale, User, UserRepo, UserRepoError};
pub use role::Role;
pub use upcoming::{
    DEFAULT_LIMIT, MAX_LIMIT, UpcomingEvent, UpcomingFilter, UpcomingKind, build_upcoming,
};
