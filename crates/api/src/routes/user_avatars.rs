//! `/users/me/avatar` — upload + delete the caller's avatar.
//!
//! Mirror of `routes::person_photos` but operating on the calling user
//! (no IDOR surface, no role check — you can only edit yourself).

use std::time::Duration;

use actix_multipart::Multipart;
use actix_web::{HttpRequest, delete, post, web};
use bytes::{Bytes, BytesMut};
use futures_util::StreamExt;
use my_family_domain::UserRepoError;
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::auth::user_claims;
use crate::images::{self, MAX_RAW_BYTES, OUTPUT_CONTENT_TYPE, OUTPUT_EXTENSION};
use crate::response::ApiResponse;
use crate::{ApiError, AppState, response_body};

const AVATAR_URL_TTL: Duration = Duration::from_hours(1);
const MAX_UPLOAD_BYTES: usize = MAX_RAW_BYTES;

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

#[allow(clippy::future_not_send)]
async fn read_single_file_field(
    mut payload: Multipart,
) -> Result<(Bytes, Option<String>), ApiError> {
    let mut bytes = BytesMut::with_capacity(64 * 1024);
    let mut filename: Option<String> = None;
    let mut seen_file = false;

    while let Some(field_res) = payload.next().await {
        let mut field =
            field_res.map_err(|e| ApiError::ImageInvalid { reason: format!("multipart: {e}") })?;
        if field.name().unwrap_or("") != "file" {
            continue;
        }
        if seen_file {
            return Err(ApiError::ImageInvalid {
                reason: "multipart contains more than one `file` field".into(),
            });
        }
        seen_file = true;
        filename = field.content_disposition().and_then(|cd| cd.get_filename().map(str::to_string));
        while let Some(chunk_res) = field.next().await {
            let chunk = chunk_res
                .map_err(|e| ApiError::ImageInvalid { reason: format!("multipart chunk: {e}") })?;
            if bytes.len().saturating_add(chunk.len()) > MAX_UPLOAD_BYTES {
                return Err(ApiError::ImageInvalid {
                    reason: format!("upload exceeds maximum size of {MAX_UPLOAD_BYTES} bytes"),
                });
            }
            bytes.extend_from_slice(&chunk);
        }
    }

    if !seen_file {
        return Err(ApiError::ImageInvalid {
            reason: "multipart body missing a `file` field".into(),
        });
    }
    Ok((bytes.freeze(), filename))
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

    // Avatar changes are user-scoped, not family-scoped — `audit_log` is
    // keyed on `family_id` so a tracing entry is the right shape.
    tracing::info!(user_id = %user_id, photo_key = %key, "user_set_avatar");

    let url = state.object_store.presigned_get(&key, AVATAR_URL_TTL).map_err(internal)?;
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

    tracing::info!(user_id = %claims.user_id.into_uuid(), "user_clear_avatar");

    Ok(ApiResponse::ok(serde_json::Value::Null))
}
