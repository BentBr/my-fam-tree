//! Core domain types: IDs, Role, capabilities. No I/O.

pub mod capabilities;
pub mod ids;
pub mod role;

pub use capabilities::{Capability, capabilities_of, has};
pub use ids::{FamilyId, FamilyMembershipId, PersonId, UserId};
pub use role::Role;
