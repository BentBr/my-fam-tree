//! `GET /families/me`, `POST /families`, `PATCH /families/{id}` and
//! `DELETE /families/{id}`.

#![expect(
    clippy::future_not_send,
    reason = "actix handlers hold !Send request types (HttpRequest, Multipart); actix runs each worker on a single thread"
)]
#![expect(
    unreachable_pub,
    reason = "actix route macros turn each handler into a pub service struct"
)]

use actix_web::{HttpRequest, HttpResponse, delete, get, patch, post, web};
use my_fam_tree_domain::{FamilyId, Role};
use uuid::Uuid;

use super::{
    CreateFamilyReq, CreateFamilyRes, CreateFamilyResponseBody, FamilyView, FamilyViewResponseBody,
    MyFamiliesRes, MyFamiliesResponseBody, RenameFamilyReq, internal, resolve_membership,
    validate_family_name,
};
use crate::cookies::access_cookie;
use crate::response::NullResponseBody;
use crate::services::audit;
use crate::services::auth_service::issue_access_token_for;
use crate::{ApiError, ApiResponse, AppState};

// ---------------------------------------------------------------------------
// GET /families/me
// ---------------------------------------------------------------------------

#[utoipa::path(
    get,
    path = "/api/v1/families/me",
    responses(
        (status = 200, description = "Caller's family memberships", body = MyFamiliesResponseBody),
        (status = 401, description = "No session"),
    ),
    security(("cookie_access" = [])),
    tag = "families",
)]
#[get("/families/me")]
pub async fn list_mine(
    state: web::Data<AppState>,
    req: HttpRequest,
) -> Result<ApiResponse<MyFamiliesRes>, ApiError> {
    let claims = crate::auth::user_claims(&req)?;
    // Fresh DB read (not the JWT claim) so we can surface `created_at` for the
    // switcher without bloating the signed token. Reflects current memberships.
    let rows = state.memberships.list_for_user(claims.user_id).await.map_err(internal)?;
    let families = rows
        .into_iter()
        .map(|m| FamilyView {
            id: m.family_id.into_uuid(),
            name: m.family_name,
            role: m.role,
            created_at: Some(m.created_at),
        })
        .collect();
    Ok(ApiResponse::ok(MyFamiliesRes { families }))
}

// ---------------------------------------------------------------------------
// POST /families
// ---------------------------------------------------------------------------

#[utoipa::path(
    post,
    path = "/api/v1/families",
    operation_id = "families_create",
    request_body = CreateFamilyReq,
    responses(
        (status = 200, description = "Family created and caller auto-joined as Owner", body = CreateFamilyResponseBody),
        (status = 401, description = "No session"),
        (status = 422, description = "Validation failed"),
    ),
    security(("cookie_access" = [])),
    tag = "families",
)]
#[post("/families")]
pub async fn create(
    state: web::Data<AppState>,
    req: HttpRequest,
    body: web::Json<CreateFamilyReq>,
) -> Result<HttpResponse, ApiError> {
    let claims = crate::auth::user_claims(&req)?;
    let name = validate_family_name(&body.name)?;

    let family = state.families.create(&name, claims.user_id).await.map_err(internal)?;
    state.memberships.insert(family.id, claims.user_id, Role::Owner).await.map_err(internal)?;
    audit::record(
        &state.audit,
        family.id,
        claims.user_id,
        "create",
        "membership",
        None,
        serde_json::json!({
            "user_id": claims.user_id.into_uuid(),
            "role": "owner",
        }),
    )
    .await;

    // Reissue access JWT so the new membership is visible immediately.
    let user = state
        .users
        .find_by_id(claims.user_id)
        .await
        .map_err(internal)?
        .ok_or(ApiError::Unauthenticated)?;
    let (access, fams) = issue_access_token_for(&state.jwt_issuer, &state.memberships, &user)
        .await
        .map_err(ApiError::Internal)?;

    let claims_payload = crate::routes::auth::ConsumeRes {
        user_id: user.id.into_uuid(),
        email: user.email.clone(),
        locale: user.locale.as_str().to_owned(),
        families: fams,
    };
    let response = CreateFamilyRes {
        family: FamilyView {
            id: family.id.into_uuid(),
            created_at: Some(family.created_at),
            name: family.name,
            role: Role::Owner,
        },
        claims: claims_payload,
    };

    let mut resp = HttpResponse::Ok().json(ApiResponse::ok(response));
    let _ = resp.add_cookie(&access_cookie(&state.cfg, access));
    Ok(resp)
}

// ---------------------------------------------------------------------------
// PATCH /families/{id}
// ---------------------------------------------------------------------------

#[utoipa::path(
    patch,
    path = "/api/v1/families/{id}",
    request_body = RenameFamilyReq,
    responses(
        (status = 200, description = "Family renamed", body = FamilyViewResponseBody),
        (status = 401, description = "No session"),
        (status = 403, description = "Insufficient role"),
        (status = 422, description = "Validation failed"),
    ),
    security(("cookie_access" = [])),
    tag = "families",
)]
#[patch("/families/{id}")]
pub async fn rename(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<Uuid>,
    body: web::Json<RenameFamilyReq>,
) -> Result<ApiResponse<FamilyView>, ApiError> {
    let claims = crate::auth::user_claims(&req)?;
    let id = path.into_inner();
    let active = resolve_membership(&claims, id)?;
    crate::auth::require_db_role(&state, claims.user_id, active.id, Role::Admin).await?;

    let name = validate_family_name(&body.name)?;
    state.families.rename(FamilyId::from_uuid(id), &name).await.map_err(internal)?;
    Ok(ApiResponse::ok(FamilyView { id, name, role: active.role, created_at: None }))
}

// ---------------------------------------------------------------------------
// DELETE /families/{id}
// ---------------------------------------------------------------------------

#[utoipa::path(
    delete,
    path = "/api/v1/families/{id}",
    responses(
        (status = 200, description = "Family deleted", body = NullResponseBody, content_type = "application/json"),
        (status = 401, description = "No session"),
        (status = 403, description = "Insufficient role"),
    ),
    security(("cookie_access" = [])),
    tag = "families",
)]
#[delete("/families/{id}")]
pub async fn delete_family(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<Uuid>,
) -> Result<ApiResponse<serde_json::Value>, ApiError> {
    let claims = crate::auth::user_claims(&req)?;
    let id = path.into_inner();
    let active = resolve_membership(&claims, id)?;
    crate::auth::require_db_role(&state, claims.user_id, active.id, Role::Owner).await?;
    state.families.delete(FamilyId::from_uuid(id)).await.map_err(internal)?;
    Ok(ApiResponse::ok(serde_json::Value::Null))
}
