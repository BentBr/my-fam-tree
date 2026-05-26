//! `/api/v1/families/{family_id}/members[/{user_id}]` — admin / owner only.
//!
//! Role matrix (enforced by [`gate_mutation`]):
//!
//! - **admin**: may revoke a `user`; may promote a `user` → `admin`. May
//!   NOT touch other admins or the owner. May not target their own row.
//! - **owner**: may revoke any non-owner; may promote/demote `user` ↔ `admin`.
//!   May not target their own row. May not set anyone's role to `owner`
//!   via PATCH — ownership transfer is a separate flow (Phase E).
//!
//! Both roles' "no self-mutation" rule applies even when the role swap
//! would otherwise be allowed. The FE filters action buttons based on
//! the same matrix; the backend is the source of truth and re-validates
//! every call.

use actix_web::{HttpRequest, delete, get, patch, web};
use chrono::{DateTime, Utc};
use my_family_domain::{FamilyId, MembershipRepoError, Role, UserId};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::auth::{require_role, user_claims_with_family};
use crate::response::ApiResponse;
use crate::services::audit;
use crate::{ApiError, AppState, response_body};

#[derive(Debug, Serialize, ToSchema)]
pub struct MemberDto {
    pub user_id: Uuid,
    pub email: String,
    pub display_name: String,
    pub role: Role,
    pub joined_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct MembersList {
    pub data: Vec<MemberDto>,
}

response_body!(pub MembersListResponseBody, MembersList);

#[derive(Debug, Deserialize, ToSchema)]
pub struct SetRoleReq {
    pub role: Role,
}

response_body!(pub MemberResponseBody, MemberDto);

fn map_db_err(target: Option<UserId>) -> impl FnOnce(MembershipRepoError) -> ApiError {
    move |e| match e {
        MembershipRepoError::NotMember => {
            ApiError::MembershipNotFound { user_id: target.map(UserId::into_uuid) }
        }
        other => ApiError::Internal(anyhow::anyhow!(other.to_string())),
    }
}

/// Role-matrix guard. Returns `Err(ApiError::InsufficientRole)` for any
/// action the active role isn't permitted to perform (including
/// self-mutation and any direct path to `owner` — ownership transfer
/// belongs to Phase E).
///
/// `new_role = None` means "DELETE this membership"; `Some(role)` means
/// "PATCH to this new role".
#[allow(
    clippy::needless_pass_by_value,
    reason = "matches Role's small Copy-ish API for readability inside the match arms"
)]
fn gate_mutation(
    actor_role: Role,
    actor_id: UserId,
    target_id: UserId,
    target_role: Role,
    new_role: Option<Role>,
) -> Result<(), ApiError> {
    if actor_id == target_id {
        return Err(ApiError::InsufficientRole { needed: actor_role });
    }
    if target_role == Role::Owner {
        // Owner row is never mutated by this route. Both admin and owner
        // are blocked; ownership transfer is its own flow.
        return Err(ApiError::InsufficientRole { needed: Role::Owner });
    }
    if matches!(new_role, Some(Role::Owner)) {
        // No path to owner via PATCH — even owner cannot promote anyone
        // to owner here. Use the dedicated owner-transfer flow.
        return Err(ApiError::InsufficientRole { needed: Role::Owner });
    }
    // Owner is the only role that can demote an admin → user. Admin can
    // promote a user → admin or revoke a user, but never touch another
    // admin's role or revoke them. Everything else is blocked, including
    // the `user` role attempting any mutation.
    let ok = match actor_role {
        Role::Owner => matches!(
            (target_role, new_role),
            (Role::User | Role::Admin, None)
                | (Role::User, Some(Role::Admin))
                | (Role::Admin, Some(Role::User))
        ),
        Role::Admin => {
            matches!((target_role, new_role), (Role::User, None | Some(Role::Admin)))
        }
        Role::User => false,
    };
    if ok {
        Ok(())
    } else {
        // The "needed" role here is best-effort guidance for the FE
        // toast — `Owner` for actions the owner could do but the admin
        // couldn't, `Admin` otherwise.
        let needed = if matches!(actor_role, Role::Admin) { Role::Owner } else { Role::Admin };
        Err(ApiError::InsufficientRole { needed })
    }
}

#[utoipa::path(
    get,
    path = "/api/v1/families/{family_id}/members",
    operation_id = "members_list",
    params(
        ("family_id" = Uuid, Path, description = "Family id (must match the active X-Family-Id)"),
    ),
    responses(
        (status = 200, description = "Members list", body = MembersListResponseBody),
        (status = 401, description = "Path family_id does not match active family"),
        (status = 403, description = "Insufficient role (admin / owner required)"),
    ),
    security(("cookie_access" = [])),
    tag = "members",
)]
#[allow(clippy::future_not_send)]
#[allow(unreachable_pub)]
#[get("/families/{family_id}/members")]
pub async fn list_members(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<Uuid>,
) -> Result<ApiResponse<MembersList>, ApiError> {
    let (_claims, active) = user_claims_with_family(&req)?;
    require_role(&active, Role::Admin)?;

    let family_id = FamilyId::from_uuid(path.into_inner());
    if active.id != family_id {
        return Err(ApiError::Unauthenticated);
    }

    let members = state.memberships.list_with_users(family_id).await.map_err(map_db_err(None))?;
    let dtos: Vec<MemberDto> = members
        .into_iter()
        .map(|m| MemberDto {
            user_id: m.user_id.into_uuid(),
            email: m.email,
            display_name: m.display_name,
            role: m.role,
            joined_at: m.joined_at,
        })
        .collect();
    Ok(ApiResponse::ok(MembersList { data: dtos }))
}

#[utoipa::path(
    patch,
    path = "/api/v1/families/{family_id}/members/{user_id}",
    operation_id = "members_set_role",
    request_body = SetRoleReq,
    params(
        ("family_id" = Uuid, Path, description = "Family id (must match the active X-Family-Id)"),
        ("user_id"   = Uuid, Path, description = "Target member's user id"),
    ),
    responses(
        (status = 200, description = "Role updated", body = MemberResponseBody),
        (status = 401, description = "Path family_id does not match active family"),
        (status = 403, description = "Role-matrix violation"),
        (status = 404, description = "Target is not a member"),
    ),
    security(("cookie_access" = [])),
    tag = "members",
)]
#[allow(clippy::future_not_send)]
#[allow(unreachable_pub)]
#[patch("/families/{family_id}/members/{user_id}")]
pub async fn set_member_role(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<(Uuid, Uuid)>,
    body: web::Json<SetRoleReq>,
) -> Result<ApiResponse<MemberDto>, ApiError> {
    let (claims, active) = user_claims_with_family(&req)?;
    require_role(&active, Role::Admin)?;

    let (family_uuid, user_uuid) = path.into_inner();
    let family_id = FamilyId::from_uuid(family_uuid);
    let target_user = UserId::from_uuid(user_uuid);
    if active.id != family_id {
        return Err(ApiError::Unauthenticated);
    }

    let target = state
        .memberships
        .find(family_id, target_user)
        .await
        .map_err(map_db_err(Some(target_user)))?
        .ok_or(ApiError::MembershipNotFound { user_id: Some(user_uuid) })?;
    gate_mutation(active.role, claims.user_id, target_user, target.role, Some(body.role))?;

    state
        .memberships
        .set_role(family_id, target_user, body.role)
        .await
        .map_err(map_db_err(Some(target_user)))?;
    audit::record(
        &state.audit,
        family_id,
        claims.user_id,
        "set_role",
        "membership",
        Some(user_uuid),
        serde_json::json!({
            "user_id": user_uuid,
            "from": target.role,
            "to": body.role,
        }),
    )
    .await;

    // Return the freshly-resolved member row by re-projecting from the
    // join. A bare `find` only gives us the role + joined_at; the FE
    // also expects email + display_name in the response shape.
    let updated = state
        .memberships
        .list_with_users(family_id)
        .await
        .map_err(map_db_err(Some(target_user)))?
        .into_iter()
        .find(|m| m.user_id == target_user)
        .ok_or(ApiError::MembershipNotFound { user_id: Some(user_uuid) })?;
    Ok(ApiResponse::ok(MemberDto {
        user_id: updated.user_id.into_uuid(),
        email: updated.email,
        display_name: updated.display_name,
        role: updated.role,
        joined_at: updated.joined_at,
    }))
}

#[utoipa::path(
    delete,
    path = "/api/v1/families/{family_id}/members/{user_id}",
    operation_id = "members_revoke",
    params(
        ("family_id" = Uuid, Path, description = "Family id (must match the active X-Family-Id)"),
        ("user_id"   = Uuid, Path, description = "Target member's user id"),
    ),
    responses(
        (status = 200, description = "Member revoked", body = crate::response::NullResponseBody),
        (status = 401, description = "Path family_id does not match active family"),
        (status = 403, description = "Role-matrix violation"),
        (status = 404, description = "Target is not a member"),
    ),
    security(("cookie_access" = [])),
    tag = "members",
)]
#[allow(clippy::future_not_send)]
#[allow(unreachable_pub)]
#[delete("/families/{family_id}/members/{user_id}")]
pub async fn revoke_member(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<(Uuid, Uuid)>,
) -> Result<ApiResponse<serde_json::Value>, ApiError> {
    let (claims, active) = user_claims_with_family(&req)?;
    require_role(&active, Role::Admin)?;

    let (family_uuid, user_uuid) = path.into_inner();
    let family_id = FamilyId::from_uuid(family_uuid);
    let target_user = UserId::from_uuid(user_uuid);
    if active.id != family_id {
        return Err(ApiError::Unauthenticated);
    }

    let target = state
        .memberships
        .find(family_id, target_user)
        .await
        .map_err(map_db_err(Some(target_user)))?
        .ok_or(ApiError::MembershipNotFound { user_id: Some(user_uuid) })?;
    gate_mutation(active.role, claims.user_id, target_user, target.role, None)?;

    state
        .memberships
        .remove(family_id, target_user)
        .await
        .map_err(map_db_err(Some(target_user)))?;
    audit::record(
        &state.audit,
        family_id,
        claims.user_id,
        "remove",
        "membership",
        Some(user_uuid),
        serde_json::json!({
            "user_id": user_uuid,
            "role": target.role,
        }),
    )
    .await;

    Ok(ApiResponse::ok(serde_json::Value::Null))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use uuid::Uuid;

    use super::{Role, UserId, gate_mutation};

    fn uid() -> UserId {
        UserId::from_uuid(Uuid::new_v4())
    }

    #[test]
    fn gate_admin_can_promote_user_to_admin() {
        let actor = uid();
        let target = uid();
        assert!(gate_mutation(Role::Admin, actor, target, Role::User, Some(Role::Admin)).is_ok(),);
    }

    #[test]
    fn gate_admin_can_revoke_user() {
        let actor = uid();
        let target = uid();
        assert!(gate_mutation(Role::Admin, actor, target, Role::User, None).is_ok());
    }

    #[test]
    fn gate_admin_cannot_demote_admin() {
        let actor = uid();
        let target = uid();
        assert!(gate_mutation(Role::Admin, actor, target, Role::Admin, Some(Role::User)).is_err(),);
    }

    #[test]
    fn gate_admin_cannot_revoke_admin() {
        let actor = uid();
        let target = uid();
        assert!(gate_mutation(Role::Admin, actor, target, Role::Admin, None).is_err());
    }

    #[test]
    fn gate_blocks_self_mutation_even_for_owner() {
        let actor = uid();
        assert!(gate_mutation(Role::Owner, actor, actor, Role::Owner, Some(Role::Admin)).is_err(),);
        assert!(gate_mutation(Role::Owner, actor, actor, Role::Owner, None).is_err());
    }

    #[test]
    fn gate_blocks_touching_owner_target() {
        let actor = uid();
        let target = uid();
        assert!(gate_mutation(Role::Owner, actor, target, Role::Owner, None).is_err());
        assert!(gate_mutation(Role::Owner, actor, target, Role::Owner, Some(Role::User)).is_err(),);
    }

    #[test]
    fn gate_blocks_promotion_to_owner_via_patch() {
        let actor = uid();
        let target = uid();
        assert!(gate_mutation(Role::Owner, actor, target, Role::Admin, Some(Role::Owner)).is_err(),);
        assert!(gate_mutation(Role::Owner, actor, target, Role::User, Some(Role::Owner)).is_err(),);
    }

    #[test]
    fn gate_owner_can_demote_admin() {
        let actor = uid();
        let target = uid();
        assert!(gate_mutation(Role::Owner, actor, target, Role::Admin, Some(Role::User)).is_ok(),);
    }

    #[test]
    fn gate_owner_can_revoke_admin() {
        let actor = uid();
        let target = uid();
        assert!(gate_mutation(Role::Owner, actor, target, Role::Admin, None).is_ok());
    }

    #[test]
    fn gate_user_role_cannot_mutate() {
        let actor = uid();
        let target = uid();
        assert!(gate_mutation(Role::User, actor, target, Role::User, None).is_err());
    }
}
