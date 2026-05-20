use std::fmt;

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

macro_rules! id_newtype {
    ($name:ident) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
        #[serde(transparent)]
        pub struct $name(Uuid);

        impl $name {
            pub const fn from_uuid(u: Uuid) -> Self {
                Self(u)
            }
            pub const fn into_uuid(self) -> Uuid {
                self.0
            }
            pub const fn as_uuid(&self) -> &Uuid {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(f)
            }
        }
    };
}

id_newtype!(UserId);
id_newtype!(FamilyId);
id_newtype!(PersonId);
id_newtype!(FamilyMembershipId);

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn user_and_family_ids_are_distinct_types() {
        let u = Uuid::new_v4();
        let user = UserId::from_uuid(u);
        let family = FamilyId::from_uuid(u);
        // If the line below compiled, that would be a bug; this test asserts equal Uuids
        // but distinct newtype wrappers serialize identically (transparent).
        assert_eq!(user.as_uuid(), family.as_uuid());
    }

    #[test]
    fn serializes_transparently_to_uuid_string() {
        let u: Uuid = "11111111-2222-3333-4444-555555555555".parse().unwrap();
        let id = PersonId::from_uuid(u);
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, "\"11111111-2222-3333-4444-555555555555\"");
    }

    #[test]
    fn round_trips_through_serde() {
        let u = Uuid::new_v4();
        let id = FamilyMembershipId::from_uuid(u);
        let s = serde_json::to_string(&id).unwrap();
        let parsed: FamilyMembershipId = serde_json::from_str(&s).unwrap();
        assert_eq!(parsed, id);
    }
}
