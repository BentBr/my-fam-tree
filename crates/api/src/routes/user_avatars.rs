//! `/users/me/avatar` — upload + delete the caller's avatar.
//!
//! Mirror of `routes::person_photos` but operating on the calling user
//! (no IDOR surface, no role check — you can only edit yourself).

use std::time::Duration;

use actix_multipart::Multipart;
use actix_web::{HttpRequest, delete, post, web};
use bytes::Bytes;
use my_fam_tree_domain::UserRepoError;
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::auth::user_claims;
use crate::images::{self, OUTPUT_CONTENT_TYPE, OUTPUT_EXTENSION};
use crate::multipart::read_single_file_field;
use crate::response::ApiResponse;
use crate::{ApiError, AppState, response_body};

const AVATAR_URL_TTL: Duration = Duration::from_hours(1);

#[derive(Debug, Serialize, ToSchema)]
pub struct UserAvatarView {
    pub avatar_key: String,
    pub avatar_url: String,
}

response_body!(pub UserAvatarResponseBody, UserAvatarView);

fn internal<E: std::fmt::Display>(e: E) -> ApiError {
    ApiError::Internal(anyhow::anyhow!(e.to_string()))
}

fn map_user_repo_err(e: UserRepoError) -> ApiError {
    match e {
        UserRepoError::NotFound => ApiError::Unauthenticated,
        UserRepoError::DuplicateEmail | UserRepoError::Db(_) => internal(e),
    }
}

fn image_err_to_api(e: &images::ImageError) -> ApiError {
    ApiError::ImageInvalid { reason: e.to_string() }
}

#[utoipa::path(
    post,
    path = "/api/v1/users/me/avatar",
    operation_id = "user_set_avatar",
    request_body(content = String, content_type = "multipart/form-data"),
    responses(
        (status = 200, description = "Avatar set", body = UserAvatarResponseBody),
        (status = 401, description = "No session"),
        (status = 422, description = "Invalid image"),
    ),
    security(("cookie_access" = [])),
    tag = "users",
)]
#[allow(clippy::future_not_send)]
#[allow(unreachable_pub)]
#[post("/users/me/avatar")]
pub async fn upload(
    state: web::Data<AppState>,
    req: HttpRequest,
    payload: Multipart,
) -> Result<ApiResponse<UserAvatarView>, ApiError> {
    let claims = user_claims(&req)?;
    let user_id = claims.user_id.into_uuid();

    let (raw, filename) = read_single_file_field(payload).await?;
    let resized =
        images::validate_and_resize(&raw, filename.as_deref()).map_err(|e| image_err_to_api(&e))?;
    let resized_bytes = Bytes::from(resized);

    let suffix = Uuid::new_v4().simple();
    let key = format!("users/{user_id}/{suffix}.{OUTPUT_EXTENSION}");

    state.object_store.put(&key, OUTPUT_CONTENT_TYPE, resized_bytes).await.map_err(internal)?;

    let previous = state
        .users
        .set_avatar_key(claims.user_id, Some(key.clone()))
        .await
        .map_err(map_user_repo_err)?;

    if let Some(old) = previous
        && old != key
        && let Err(e) = state.object_store.delete(&old).await
    {
        tracing::warn!(error = ?e, previous_key = %old, "failed to delete previous avatar from object store");
    }

    // Propagate the new avatar to every person row across every family
    // where `linked_user_id = self`. Latest-write-wins: this overwrites any
    // individual person-photo overrides, which is the simple semantic
    // requested ("the account image is setting images top-down…without
    // more logic"). A subsequent person-photo upload still wins for that
    // one person; another account-avatar update will broadcast again.
    let touched = state
        .persons
        .set_photo_key_for_linked_user(claims.user_id, Some(key.clone()))
        .await
        .map_err(|e| internal(format!("propagate avatar to linked persons: {e}")))?;
    tracing::info!(user_id = %user_id, photo_key = %key, propagated_to = touched, "user_set_avatar");

    let url = state.object_store.presigned_get(&key, AVATAR_URL_TTL).await.map_err(internal)?;
    Ok(ApiResponse::ok(UserAvatarView { avatar_key: key, avatar_url: url }))
}

#[utoipa::path(
    delete,
    path = "/api/v1/users/me/avatar",
    operation_id = "user_clear_avatar",
    responses(
        (status = 200, description = "Avatar cleared", body = crate::response::NullResponseBody),
        (status = 401, description = "No session"),
    ),
    security(("cookie_access" = [])),
    tag = "users",
)]
#[allow(clippy::future_not_send)]
#[allow(unreachable_pub)]
#[delete("/users/me/avatar")]
pub async fn clear(
    state: web::Data<AppState>,
    req: HttpRequest,
) -> Result<ApiResponse<serde_json::Value>, ApiError> {
    let claims = user_claims(&req)?;

    let previous =
        state.users.set_avatar_key(claims.user_id, None).await.map_err(map_user_repo_err)?;

    if let Some(old) = previous
        && let Err(e) = state.object_store.delete(&old).await
    {
        tracing::warn!(error = ?e, previous_key = %old, "failed to delete avatar from object store");
    }

    // Same broadcast as the upload path, in reverse — clear photo_key on
    // every linked person row so the propagated avatars disappear in
    // lockstep with the account avatar.
    let touched = state
        .persons
        .set_photo_key_for_linked_user(claims.user_id, None)
        .await
        .map_err(|e| internal(format!("clear avatar across linked persons: {e}")))?;
    tracing::info!(user_id = %claims.user_id.into_uuid(), propagated_to = touched, "user_clear_avatar");

    Ok(ApiResponse::ok(serde_json::Value::Null))
}
