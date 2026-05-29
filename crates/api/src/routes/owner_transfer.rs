//! `/api/v1/families/{family_id}/transfer-owner[/confirm]` — Phase E
//! double-verification ownership handoff.
//!
//! Four routes:
//!
//! - `POST   /families/{id}/transfer-owner`         — owner only. Body
//!   `{to_user_id}`. Verifies the target is a current `Admin` in the family,
//!   generates two opaque tokens (one per side), inserts the pending transfer
//!   with a 1-hour expiry, and sends two emails.
//! - `POST   /families/{id}/transfer-owner/confirm` — token-bearing. Body
//!   `{token}`. Does NOT bind to `X-Family-Id` since the recipient may have a
//!   different active family in their session; the BE derives `family_id`
//!   from the matched transfer row. Confirms the relevant side; if BOTH
//!   sides are now confirmed, atomically demotes the previous owner to
//!   admin, promotes the target to owner, and marks the transfer complete.
//! - `DELETE /families/{id}/transfer-owner`         — owner only, cancels
//!   the pending transfer.
//! - `GET    /families/{id}/transfer-owner`         — admin+owner, returns
//!   the active transfer (or `null`).
//!
//! Audit entries fire at every transition: `begin`, `confirm` (×2),
//! `complete`, `cancel`. See `plan-corrections.md` §9 for the slug
//! convention.

use actix_web::{HttpRequest, delete, get, post, web};
use chrono::{Duration, Utc};
use my_family_domain::{FamilyId, MembershipRepoError, OwnerTransferRepoError, Role, UserId};
use my_family_email::{
    Locale as EmailLocale, render_owner_transfer_admin, render_owner_transfer_owner,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::auth::{
    generate_opaque_token, hash_token, require_role, user_claims, user_claims_with_family,
};
use crate::services::audit;
use crate::validation::role_invalid;
use crate::{ApiError, ApiResponse, AppState, response_body};

// ---------------------------------------------------------------------------
// DTOs.
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct BeginReq {
    pub to_user_id: Uuid,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ConfirmReq {
    pub token: String,
}

/// Wire DTO for both single-transfer endpoints (`POST` begin / `POST` confirm
/// / `GET` status). `null` from `GET` is rendered as the outer `data: null`.
#[derive(Debug, Serialize, ToSchema)]
pub struct TransferStatus {
    pub id: Uuid,
    pub from_user_id: Uuid,
    pub to_user_id: Uuid,
    pub from_confirmed: bool,
    pub to_confirmed: bool,
    pub expires_at: chrono::DateTime<Utc>,
}

response_body!(pub TransferStatusResponseBody, TransferStatus);
response_body!(pub TransferStatusOptionalResponseBody, Option<TransferStatus>);

fn internal<E: std::fmt::Display>(e: E) -> ApiError {
    ApiError::Internal(anyhow::anyhow!(e.to_string()))
}

/// Render + dispatch both transfer-confirmation emails. Kept as a helper so
/// the `begin` handler stays under the 100-line clippy budget.
#[allow(clippy::future_not_send, clippy::too_many_arguments)]
async fn send_transfer_emails(
    state: &AppState,
    family_name: &str,
    from_email: &str,
    from_display_name: &str,
    from_locale_str: &str,
    from_link: &str,
    to_email: &str,
    to_display_name: &str,
    to_locale_str: &str,
    to_link: &str,
) -> Result<(), ApiError> {
    let from_locale = EmailLocale::from_str_or_en(from_locale_str);
    let to_locale = EmailLocale::from_str_or_en(to_locale_str);
    let (from_subject, from_body) =
        render_owner_transfer_owner(from_locale, family_name, to_display_name, from_link)
            .map_err(internal)?;
    let (to_subject, to_body) = render_owner_transfer_admin(
        to_locale,
        family_name,
        from_display_name,
        to_display_name,
        to_link,
    )
    .map_err(internal)?;

    // Both confirmation emails go through the durable outbox; the worker
    // drains via SMTP with retry. The outbox row only carries to_addr —
    // the SMTP `To:` display name (`Some(from_display_name)`) used to be
    // a nicety on the synchronous path and isn't captured today.
    let _ = (from_display_name, to_display_name);
    state
        .outbox
        .enqueue(&my_family_domain::EmailOutboxInsert {
            kind: my_family_domain::EmailOutboxKind::OWNER_TRANSFER_FROM.to_string(),
            to_addr: from_email.to_owned(),
            subject: from_subject,
            text_body: from_body,
            html_body: None,
        })
        .await
        .map_err(internal)?;
    state
        .outbox
        .enqueue(&my_family_domain::EmailOutboxInsert {
            kind: my_family_domain::EmailOutboxKind::OWNER_TRANSFER_TO.to_string(),
            to_addr: to_email.to_owned(),
            subject: to_subject,
            text_body: to_body,
            html_body: None,
        })
        .await
        .map_err(internal)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// POST /families/{id}/transfer-owner
// ---------------------------------------------------------------------------

#[utoipa::path(
    post,
    path = "/api/v1/families/{family_id}/transfer-owner",
    operation_id = "owner_transfer_begin",
    request_body = BeginReq,
    params(
        ("family_id" = Uuid, Path, description = "Family id (must match the active X-Family-Id)"),
    ),
    responses(
        (status = 200, description = "Transfer started; both confirmation emails dispatched", body = TransferStatusResponseBody),
        (status = 401, description = "Path family_id does not match active family"),
        (status = 403, description = "Owner role required"),
        (status = 409, description = "A transfer is already pending"),
        (status = 422, description = "Target is not a current admin in this family"),
    ),
    security(("cookie_access" = [])),
    tag = "owner-transfer",
)]
#[allow(clippy::future_not_send)]
#[allow(unreachable_pub)]
#[post("/families/{family_id}/transfer-owner")]
pub async fn begin(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<Uuid>,
    body: web::Json<BeginReq>,
) -> Result<ApiResponse<TransferStatus>, ApiError> {
    let (claims, active) = user_claims_with_family(&req)?;
    require_role(&active, Role::Owner)?;

    let family_id = FamilyId::from_uuid(path.into_inner());
    if active.id != family_id {
        return Err(ApiError::Unauthenticated);
    }

    let target = UserId::from_uuid(body.to_user_id);
    if target == claims.user_id {
        return Err(role_invalid("/to_user_id", "cannot transfer ownership to yourself"));
    }

    // Target must already be an admin member of this family.
    let target_membership = state
        .memberships
        .find(family_id, target)
        .await
        .map_err(|e| match e {
            MembershipRepoError::NotMember => {
                ApiError::MembershipNotFound { user_id: Some(body.to_user_id) }
            }
            other => internal(other),
        })?
        .ok_or(ApiError::MembershipNotFound { user_id: Some(body.to_user_id) })?;
    if target_membership.role != Role::Admin {
        return Err(role_invalid("/to_user_id", "target must be an admin in this family"));
    }

    let (from_token, from_hash) = generate_opaque_token();
    let (to_token, to_hash) = generate_opaque_token();
    let expires_at = Utc::now() + Duration::hours(1);

    let id = state
        .owner_transfers
        .begin(family_id, claims.user_id, target, &from_hash, &to_hash, expires_at)
        .await
        .map_err(|e| match e {
            OwnerTransferRepoError::AlreadyPending => ApiError::OwnerTransferPending,
            other => internal(other),
        })?;

    // Resolve display names + family name for the email bodies.
    let from_user = state
        .users
        .find_by_id(claims.user_id)
        .await
        .map_err(internal)?
        .ok_or(ApiError::Unauthenticated)?;
    let to_user = state
        .users
        .find_by_id(target)
        .await
        .map_err(internal)?
        .ok_or(ApiError::MembershipNotFound { user_id: Some(body.to_user_id) })?;
    let family = state
        .families
        .find_by_id(family_id)
        .await
        .map_err(internal)?
        .ok_or_else(|| internal("active family missing"))?;

    let from_link =
        format!("{}/account/owner-transfer/confirm?token={}", state.cfg.web.public_url, from_token);
    let to_link =
        format!("{}/account/owner-transfer/confirm?token={}", state.cfg.web.public_url, to_token);
    send_transfer_emails(
        &state,
        &family.name,
        &from_user.email,
        &from_user.display_name,
        from_user.locale.as_str(),
        &from_link,
        &to_user.email,
        &to_user.display_name,
        to_user.locale.as_str(),
        &to_link,
    )
    .await?;

    audit::record(
        &state.audit,
        family_id,
        claims.user_id,
        "begin",
        "owner_transfer",
        Some(id),
        serde_json::json!({ "to_user_id": body.to_user_id }),
    )
    .await;

    Ok(ApiResponse::ok(TransferStatus {
        id,
        from_user_id: claims.user_id.into_uuid(),
        to_user_id: body.to_user_id,
        from_confirmed: false,
        to_confirmed: false,
        expires_at,
    }))
}

// ---------------------------------------------------------------------------
// POST /families/{id}/transfer-owner/confirm
// ---------------------------------------------------------------------------

/// Token-bearing: the supplied `token` IS the authorization. We require an
/// authenticated session (so the audit row has an `actor_user_id`) but do
/// NOT bind to `X-Family-Id`, because the recipient may have a different
/// active family selected when they click the link from their email.
#[utoipa::path(
    post,
    path = "/api/v1/families/{family_id}/transfer-owner/confirm",
    operation_id = "owner_transfer_confirm",
    request_body = ConfirmReq,
    params(
        ("family_id" = Uuid, Path, description = "Family id (informational; the BE derives it from the token hash)"),
    ),
    responses(
        (status = 200, description = "One side confirmed; completes the transfer if both sides agreed", body = TransferStatusResponseBody),
        (status = 401, description = "No session"),
        (status = 404, description = "Token not found / already used"),
        (status = 410, description = "Token expired"),
    ),
    security(("cookie_access" = [])),
    tag = "owner-transfer",
)]
#[allow(clippy::future_not_send)]
#[allow(unreachable_pub)]
#[post("/families/{family_id}/transfer-owner/confirm")]
pub async fn confirm(
    state: web::Data<AppState>,
    req: HttpRequest,
    _path: web::Path<Uuid>,
    body: web::Json<ConfirmReq>,
) -> Result<ApiResponse<TransferStatus>, ApiError> {
    let claims = user_claims(&req)?;
    let token_hash = hash_token(body.token.trim());
    let now = Utc::now();

    let (transfer, side) =
        state.owner_transfers.confirm(&token_hash, now).await.map_err(|e| match e {
            OwnerTransferRepoError::NotFound => ApiError::PersonNotFound { id: None },
            OwnerTransferRepoError::Expired => ApiError::InviteExpired,
            other => internal(other),
        })?;

    let family_id = transfer.family_id;

    audit::record(
        &state.audit,
        family_id,
        claims.user_id,
        "confirm",
        "owner_transfer",
        Some(transfer.id),
        serde_json::json!({ "side": side.as_str() }),
    )
    .await;

    // If both sides are now confirmed, swap the roles + mark complete.
    // Order: demote the previous owner -> admin BEFORE promoting the target
    // -> owner. A future invariant could add a partial unique index on
    // `(family_id, role = owner)`; this order is safe under either.
    let both_confirmed = transfer.from_confirmed_at.is_some() && transfer.to_confirmed_at.is_some();
    if both_confirmed && transfer.completed_at.is_none() {
        state
            .memberships
            .set_role(family_id, transfer.from_user_id, Role::Admin)
            .await
            .map_err(internal)?;
        state
            .memberships
            .set_role(family_id, transfer.to_user_id, Role::Owner)
            .await
            .map_err(internal)?;
        state.owner_transfers.complete(transfer.id, now).await.map_err(internal)?;
        audit::record(
            &state.audit,
            family_id,
            claims.user_id,
            "complete",
            "owner_transfer",
            Some(transfer.id),
            serde_json::json!({
                "from_user_id": transfer.from_user_id.into_uuid(),
                "to_user_id":   transfer.to_user_id.into_uuid(),
            }),
        )
        .await;
    }

    Ok(ApiResponse::ok(TransferStatus {
        id: transfer.id,
        from_user_id: transfer.from_user_id.into_uuid(),
        to_user_id: transfer.to_user_id.into_uuid(),
        from_confirmed: transfer.from_confirmed_at.is_some(),
        to_confirmed: transfer.to_confirmed_at.is_some(),
        expires_at: transfer.expires_at,
    }))
}

// ---------------------------------------------------------------------------
// DELETE /families/{id}/transfer-owner — owner cancels.
// ---------------------------------------------------------------------------

#[utoipa::path(
    delete,
    path = "/api/v1/families/{family_id}/transfer-owner",
    operation_id = "owner_transfer_cancel",
    params(
        ("family_id" = Uuid, Path, description = "Family id (must match the active X-Family-Id)"),
    ),
    responses(
        (status = 200, description = "Pending transfer cancelled", body = crate::response::NullResponseBody),
        (status = 401, description = "Path family_id does not match active family"),
        (status = 403, description = "Owner role required"),
    ),
    security(("cookie_access" = [])),
    tag = "owner-transfer",
)]
#[allow(clippy::future_not_send)]
#[allow(unreachable_pub)]
#[delete("/families/{family_id}/transfer-owner")]
pub async fn cancel(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<Uuid>,
) -> Result<ApiResponse<serde_json::Value>, ApiError> {
    let (claims, active) = user_claims_with_family(&req)?;
    require_role(&active, Role::Owner)?;
    let family_id = FamilyId::from_uuid(path.into_inner());
    if active.id != family_id {
        return Err(ApiError::Unauthenticated);
    }
    state.owner_transfers.cancel(family_id, Utc::now()).await.map_err(internal)?;
    audit::record(
        &state.audit,
        family_id,
        claims.user_id,
        "cancel",
        "owner_transfer",
        None,
        serde_json::json!({}),
    )
    .await;
    Ok(ApiResponse::ok(serde_json::Value::Null))
}

// ---------------------------------------------------------------------------
// GET /families/{id}/transfer-owner — show pending status (admin+owner).
// ---------------------------------------------------------------------------

#[utoipa::path(
    get,
    path = "/api/v1/families/{family_id}/transfer-owner",
    operation_id = "owner_transfer_status",
    params(
        ("family_id" = Uuid, Path, description = "Family id (must match the active X-Family-Id)"),
    ),
    responses(
        (status = 200, description = "Pending transfer or null", body = TransferStatusOptionalResponseBody),
        (status = 401, description = "Path family_id does not match active family"),
        (status = 403, description = "Insufficient role (admin / owner required)"),
    ),
    security(("cookie_access" = [])),
    tag = "owner-transfer",
)]
#[allow(clippy::future_not_send)]
#[allow(unreachable_pub)]
#[get("/families/{family_id}/transfer-owner")]
pub async fn status(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<Uuid>,
) -> Result<ApiResponse<Option<TransferStatus>>, ApiError> {
    let (_claims, active) = user_claims_with_family(&req)?;
    require_role(&active, Role::Admin)?;
    let family_id = FamilyId::from_uuid(path.into_inner());
    if active.id != family_id {
        return Err(ApiError::Unauthenticated);
    }
    let pending = state.owner_transfers.find_active(family_id).await.map_err(internal)?;
    let dto = pending.map(|t| TransferStatus {
        id: t.id,
        from_user_id: t.from_user_id.into_uuid(),
        to_user_id: t.to_user_id.into_uuid(),
        from_confirmed: t.from_confirmed_at.is_some(),
        to_confirmed: t.to_confirmed_at.is_some(),
        expires_at: t.expires_at,
    });
    Ok(ApiResponse::ok(dto))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn begin_req_deserialises_target_uuid() {
        let r: BeginReq = serde_json::from_value(
            serde_json::json!({"to_user_id": "00000000-0000-0000-0000-000000000001"}),
        )
        .unwrap();
        assert_eq!(r.to_user_id.to_string(), "00000000-0000-0000-0000-000000000001");
    }

    #[test]
    fn confirm_req_deserialises_token() {
        let r: ConfirmReq = serde_json::from_value(serde_json::json!({"token": "abc"})).unwrap();
        assert_eq!(r.token, "abc");
    }
}
