//! SQLx-backed repositories. No business logic here.

pub mod error;
pub mod families;
pub mod family_invites;
pub mod family_memberships;
pub mod magic_link_tokens;
pub mod parent_links;
pub mod partnerships;
pub mod persons;
pub mod pool;
pub mod refresh_tokens;
pub mod users;

pub use error::PersistenceError;
pub use families::PgFamilyRepo;
pub use family_invites::PgFamilyInviteRepo;
pub use family_memberships::PgFamilyMembershipRepo;
pub use magic_link_tokens::PgMagicLinkRepo;
pub use parent_links::PgParentLinkRepo;
pub use partnerships::PgPartnershipRepo;
pub use persons::PgPersonRepo;
pub use pool::Database;
pub use refresh_tokens::PgRefreshTokenRepo;
pub use users::PgUserRepo;
