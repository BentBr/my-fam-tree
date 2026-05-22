use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    User,
    Admin,
    Owner,
}

impl Role {
    #[must_use]
    pub const fn at_least(self, needed: Self) -> bool {
        self.rank() >= needed.rank()
    }

    const fn rank(self) -> u8 {
        match self {
            Self::User => 1,
            Self::Admin => 2,
            Self::Owner => 3,
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn role_serializes_snake_case() {
        assert_eq!(serde_json::to_string(&Role::User).unwrap(), "\"user\"");
        assert_eq!(serde_json::to_string(&Role::Admin).unwrap(), "\"admin\"");
        assert_eq!(serde_json::to_string(&Role::Owner).unwrap(), "\"owner\"");
    }

    #[test]
    fn at_least_owner_only_satisfied_by_owner() {
        assert!(!Role::User.at_least(Role::Owner));
        assert!(!Role::Admin.at_least(Role::Owner));
        assert!(Role::Owner.at_least(Role::Owner));
    }

    #[test]
    fn at_least_admin_satisfied_by_admin_and_owner() {
        assert!(!Role::User.at_least(Role::Admin));
        assert!(Role::Admin.at_least(Role::Admin));
        assert!(Role::Owner.at_least(Role::Admin));
    }
}
