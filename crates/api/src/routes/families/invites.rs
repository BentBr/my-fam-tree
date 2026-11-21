//! `POST /families/{id}/invites` — issue + email a single-use invite.

use actix_web::{HttpRequest, post, web};
use my_fam_tree_domain::{FamilyId, Role};
use my_fam_tree_email::{Locale as EmailLocale, render_invite};
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

use super::{InviteReq, InviteRes, InviteResponseBody, internal, resolve_membership, seconds_i64};
use crate::auth::generate_opaque_token;
use crate::services::audit;
use crate::validation::{email_invalid, looks_like_email, role_invalid};
use crate::{ApiError, ApiResponse, AppState};

// ---------------------------------------------------------------------------
// POST /families/{id}/invites
// ---------------------------------------------------------------------------

#[utoipa::path(
    post,
    path = "/api/v1/families/{id}/invites",
    request_body = InviteReq,
    responses(
        (status = 200, description = "Invite created and email queued", body = InviteResponseBody),
        (status = 401, description = "No session"),
        (status = 403, description = "Insufficient role"),
        (status = 422, description = "Validation failed"),
    ),
    security(("cookie_access" = [])),
    tag = "families",
)]
#[allow(clippy::future_not_send)]
#[allow(unreachable_pub)]
#[post("/families/{id}/invites")]
pub async fn invite(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<Uuid>,
    body: web::Json<InviteReq>,
) -> Result<ApiResponse<InviteRes>, ApiError> {
    let claims = crate::auth::user_claims(&req)?;
    let family_id = path.into_inner();
    let active = resolve_membership(&claims, family_id)?;
    crate::auth::require_db_role(&state, claims.user_id, active.id, Role::Admin).await?;

    if body.role == Role::Owner {
        return Err(role_invalid("/role", "cannot invite as owner"));
    }

    let email = body.email.trim().to_lowercase();
    if !looks_like_email(&email) {
        return Err(email_invalid("/email"));
    }

    // Reject duplicates BEFORE generating a token / sending email. Two
    // pending invites for the same email in the same family would race on
    // accept (only one can match the freshly-signed-in JWT email) and
    // pollute the admin pending-list with stale rows.
    let fid = FamilyId::from_uuid(family_id);
    if state.invites.find_pending_by_email(fid, &email).await.map_err(internal)?.is_some() {
        return Err(ApiError::InviteDuplicate);
    }

    // Cross-family IDOR guard (security audit MEDIUM). `family_invites.person_id`
    // only `REFERENCES persons(id)`, not the invite's family_id — an F1
    // admin could otherwise mint an invite referencing an F2 person. On
    // accept the linkage step would then fail (404 mapped to 500) but the
    // membership row + audit "verify" event would already have landed,
    // leaving a member without the intended linkage. Reject at create time.
    if let Some(pid) = body.person_id
        && state
            .persons
            .find_in_family(fid, my_fam_tree_domain::PersonId::from_uuid(pid))
            .await
            .map_err(internal)?
            .is_none()
    {
        return Err(ApiError::PersonNotFound { id: Some(pid) });
    }

    let (token, hash) = generate_opaque_token();
    let invite_id = state
        .invites
        .create(
            fid,
            &email,
            body.role,
            claims.user_id,
            body.person_id,
            &hash,
            OffsetDateTime::now_utc()
                + Duration::seconds(seconds_i64(state.cfg.magic_link.invite_ttl_seconds)),
        )
        .await
        .map_err(internal)?;

    // Locale-prefixed for the inviter's language (the invitee may not have an
    // account yet, so the inviter's preference is the best signal we have).
    let locale = EmailLocale::from_str_or_en(&claims.locale);
    let link = locale.link(&state.cfg.web.public_url, &format!("/invite/accept?token={token}"));
    let email_msg =
        render_invite(locale, &state.cfg.web.public_url, &active.name, &claims.email, &link)
            .map_err(internal)?;
    // Enqueue into the durable outbox — the worker drains via SMTP. Same
    // request thread used to block on SmtpSender.send(); now it returns as
    // soon as Postgres has the row.
    state
        .outbox
        .enqueue(&my_fam_tree_domain::EmailOutboxInsert {
            kind: my_fam_tree_domain::EmailOutboxKind::INVITE.to_string(),
            to_addr: email.clone(),
            subject: email_msg.subject,
            text_body: email_msg.text,
            html_body: Some(email_msg.html),
        })
        .await
        .map_err(internal)?;

    audit::record(
        &state.audit,
        fid,
        claims.user_id,
        "invite",
        "membership",
        Some(invite_id),
        serde_json::json!({
            "email": email,
            "role": body.role,
            "person_id": body.person_id,
        }),
    )
    .await;
    Ok(ApiResponse::ok(InviteRes { status: "sent" }))
}
