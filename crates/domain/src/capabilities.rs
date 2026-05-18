use crate::role::Role;

/// Every action the API authorizes against. Adding a variant forces every role
/// mapping to be updated (exhaustiveness check in `capabilities_of`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Capability {
    // Self-service
    EditOwnPerson,
    EditOwnContacts,
    ManageOwnReminders,
    // Family content
    CreatePerson,
    EditAnyPerson,
    DeletePerson,
    ManageRelationships,
    EditAnyContact,
    // Family administration
    InviteUsers,
    ManageRolesBelowOwner,
    // Ownership
    TransferOwnership,
    DeleteFamily,
}

/// Static capability set for a role. Pure data, no DB lookup.
#[allow(clippy::enum_glob_use)] // idiomatic within a single match over a local enum
pub const fn capabilities_of(role: Role) -> &'static [Capability] {
    use Capability::*;
    match role {
        Role::User => &[EditOwnPerson, EditOwnContacts, ManageOwnReminders],
        Role::Admin => &[
            EditOwnPerson,
            EditOwnContacts,
            ManageOwnReminders,
            CreatePerson,
            EditAnyPerson,
            DeletePerson,
            ManageRelationships,
            EditAnyContact,
            InviteUsers,
            ManageRolesBelowOwner,
        ],
        Role::Owner => &[
            EditOwnPerson,
            EditOwnContacts,
            ManageOwnReminders,
            CreatePerson,
            EditAnyPerson,
            DeletePerson,
            ManageRelationships,
            EditAnyContact,
            InviteUsers,
            ManageRolesBelowOwner,
            TransferOwnership,
            DeleteFamily,
        ],
    }
}

// Can't be `const fn`: slice `.contains` requires PartialEq which isn't const yet.
#[allow(clippy::missing_const_for_fn)]
pub fn has(role: Role, cap: Capability) -> bool {
    capabilities_of(role).contains(&cap)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_cannot_invite() {
        assert!(!has(Role::User, Capability::InviteUsers));
    }

    #[test]
    fn admin_can_invite_but_cannot_delete_family() {
        assert!(has(Role::Admin, Capability::InviteUsers));
        assert!(!has(Role::Admin, Capability::DeleteFamily));
    }

    #[test]
    fn owner_can_do_everything_admin_can_plus_delete_family() {
        for cap in capabilities_of(Role::Admin) {
            assert!(has(Role::Owner, *cap), "owner missing {cap:?}");
        }
        assert!(has(Role::Owner, Capability::DeleteFamily));
        assert!(has(Role::Owner, Capability::TransferOwnership));
    }

    #[test]
    fn every_role_includes_self_service() {
        for role in [Role::User, Role::Admin, Role::Owner] {
            assert!(has(role, Capability::EditOwnPerson));
            assert!(has(role, Capability::EditOwnContacts));
            assert!(has(role, Capability::ManageOwnReminders));
        }
    }
}
