//! Locked-down error contract.
//!
//! Every handler returns `Result<ApiResponse<T>, ApiError>`. Errors serialize
//! as `RFC 7807` problem+json with a stable machine-readable `code` enum.
//! Internal errors are sanitized for the wire and logged via tracing with the
//! full chain.

use actix_web::body::BoxBody;
use actix_web::http::StatusCode;
use actix_web::http::header::{CONTENT_TYPE, HeaderValue, RETRY_AFTER};
use actix_web::{HttpResponse, ResponseError};
use my_family_domain::Role;
use serde::Serialize;
use thiserror::Error;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Serialize, ToSchema)]
pub struct ApiErrorBody {
    #[serde(rename = "type")]
    pub type_: String,
    pub title: &'static str,
    pub status: u16,
    pub code: ErrorCode,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instance: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fields: Option<Vec<FieldViolation>>,
}

/// A single field validation failure on the wire.
///
/// - `path`: JSON Pointer or query/header path to the offending field (e.g.
///   `/email`, `/body/family_name`, `/headers/x-family-id`).
/// - `code`: stable i18n key (e.g. `validation.email_invalid`,
///   `validation.string_too_long`). FE looks this up in en/de catalogs.
/// - `message`: English fallback so non-FE clients still get a readable
///   error.
/// - `params`: named placeholders FE substitutes into the localized
///   message. Always serialized (even when empty) so FE clients don't have
///   to guard `params?.max`. Use e.g. `{"max": 100, "actual": 105}`.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct FieldViolation {
    pub path: String,
    pub code: String,
    pub message: String,
    #[serde(default)]
    pub params: std::collections::BTreeMap<String, serde_json::Value>,
}

impl FieldViolation {
    /// Convenience: create a violation with no `params`. Use this when the
    /// localized message doesn't need any runtime substitution.
    pub fn new(
        path: impl Into<String>,
        code: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            path: path.into(),
            code: code.into(),
            message: message.into(),
            params: std::collections::BTreeMap::new(),
        }
    }

    /// Builder-style: attach a single named parameter.
    #[must_use]
    pub fn with_param(
        mut self,
        key: impl Into<String>,
        value: impl Into<serde_json::Value>,
    ) -> Self {
        self.params.insert(key.into(), value.into());
        self
    }
}

#[derive(Debug, Clone, Copy, Serialize, ToSchema, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    AuthUnauthenticated,
    AuthTokenExpired,
    AuthTokenInvalid,
    AuthRefreshInvalid,
    AuthMagicLinkInvalid,
    FamilyNotMember,
    FamilyInsufficientRole,
    FamilyInviteExpired,
    PersonNotFound,
    PersonNotEditable,
    ContactNotFound,
    ContactNotEditable,
    ReminderNotFound,
    RelationshipCycle,
    PartnershipDuplicate,
    ParentLinkDuplicate,
    ValidationFailed,
    ConflictStale,
    EmailTaken,
    RateLimited,
    Upstream,
    Internal,
}

impl ErrorCode {
    #[must_use]
    pub const fn http_status(self) -> StatusCode {
        match self {
            Self::AuthUnauthenticated
            | Self::AuthTokenExpired
            | Self::AuthTokenInvalid
            | Self::AuthRefreshInvalid
            | Self::AuthMagicLinkInvalid => StatusCode::UNAUTHORIZED,
            Self::FamilyNotMember
            | Self::FamilyInsufficientRole
            | Self::PersonNotEditable
            | Self::ContactNotEditable => StatusCode::FORBIDDEN,
            Self::FamilyInviteExpired => StatusCode::GONE,
            Self::PersonNotFound | Self::ContactNotFound | Self::ReminderNotFound => {
                StatusCode::NOT_FOUND
            }
            Self::RelationshipCycle
            | Self::PartnershipDuplicate
            | Self::ParentLinkDuplicate
            | Self::ConflictStale
            | Self::EmailTaken => StatusCode::CONFLICT,
            Self::ValidationFailed => StatusCode::UNPROCESSABLE_ENTITY,
            Self::RateLimited => StatusCode::TOO_MANY_REQUESTS,
            Self::Upstream => StatusCode::BAD_GATEWAY,
            Self::Internal => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    #[must_use]
    pub const fn title(self) -> &'static str {
        match self {
            Self::AuthUnauthenticated => "Authentication required",
            Self::AuthTokenExpired => "Access token expired",
            Self::AuthTokenInvalid => "Access token invalid",
            Self::AuthRefreshInvalid => "Refresh token invalid",
            Self::AuthMagicLinkInvalid => "Magic link invalid",
            Self::FamilyNotMember => "Not a member of this family",
            Self::FamilyInsufficientRole => "Insufficient role for this action",
            Self::FamilyInviteExpired => "Invite expired",
            Self::PersonNotFound => "Person not found",
            Self::PersonNotEditable => "You may only edit your own profile",
            Self::ContactNotFound => "Contact not found",
            Self::ContactNotEditable => "You may only edit contacts on your own profile",
            Self::ReminderNotFound => "Reminder not found",
            Self::RelationshipCycle => "Relationship would create a cycle",
            Self::PartnershipDuplicate => "Partnership already exists",
            Self::ParentLinkDuplicate => "Parent link already exists",
            Self::ValidationFailed => "Validation failed",
            Self::ConflictStale => "Stale state",
            Self::EmailTaken => "Email already in use",
            Self::RateLimited => "Rate limited",
            Self::Upstream => "Upstream service error",
            Self::Internal => "Internal server error",
        }
    }

    #[must_use]
    pub const fn slug(self) -> &'static str {
        match self {
            Self::AuthUnauthenticated => "auth.unauthenticated",
            Self::AuthTokenExpired => "auth.token_expired",
            Self::AuthTokenInvalid => "auth.token_invalid",
            Self::AuthRefreshInvalid => "auth.refresh_invalid",
            Self::AuthMagicLinkInvalid => "auth.magic_link_invalid",
            Self::FamilyNotMember => "family.not_member",
            Self::FamilyInsufficientRole => "family.insufficient_role",
            Self::FamilyInviteExpired => "family.invite_expired",
            Self::PersonNotFound => "person.not_found",
            Self::PersonNotEditable => "person.not_editable",
            Self::ContactNotFound => "contact.not_found",
            Self::ContactNotEditable => "contact.not_editable",
            Self::ReminderNotFound => "reminder.not_found",
            Self::RelationshipCycle => "relationship.cycle",
            Self::PartnershipDuplicate => "partnership.duplicate",
            Self::ParentLinkDuplicate => "parent_link.duplicate",
            Self::ValidationFailed => "validation.failed",
            Self::ConflictStale => "conflict.stale",
            Self::EmailTaken => "email.taken",
            Self::RateLimited => "rate_limited",
            Self::Upstream => "upstream",
            Self::Internal => "internal",
        }
    }

    pub const ALL: &'static [Self] = &[
        Self::AuthUnauthenticated,
        Self::AuthTokenExpired,
        Self::AuthTokenInvalid,
        Self::AuthRefreshInvalid,
        Self::AuthMagicLinkInvalid,
        Self::FamilyNotMember,
        Self::FamilyInsufficientRole,
        Self::FamilyInviteExpired,
        Self::PersonNotFound,
        Self::PersonNotEditable,
        Self::ContactNotFound,
        Self::ContactNotEditable,
        Self::ReminderNotFound,
        Self::RelationshipCycle,
        Self::PartnershipDuplicate,
        Self::ParentLinkDuplicate,
        Self::ValidationFailed,
        Self::ConflictStale,
        Self::EmailTaken,
        Self::RateLimited,
        Self::Upstream,
        Self::Internal,
    ];
}

#[derive(Debug, Error)]
pub enum ApiError {
    #[error("authentication required")]
    Unauthenticated,
    #[error("access token expired")]
    TokenExpired,
    #[error("access token invalid")]
    TokenInvalid,
    #[error("refresh token invalid")]
    RefreshInvalid,
    #[error("magic link invalid or expired")]
    MagicLinkInvalid,
    #[error("not a member of family {0}")]
    NotFamilyMember(Uuid),
    #[error("insufficient role: needs {needed:?}")]
    InsufficientRole { needed: Role },
    #[error("invite expired")]
    InviteExpired,
    #[error("person not found")]
    PersonNotFound { id: Option<Uuid> },
    #[error("you may only edit your own profile")]
    PersonNotEditable,
    #[error("contact not found")]
    ContactNotFound { id: Option<Uuid> },
    #[error("you may only edit contacts on your own profile")]
    ContactNotEditable,
    #[error("reminder not found")]
    ReminderNotFound,
    #[error("relationship would create a cycle")]
    RelationshipCycle,
    #[error("partnership already exists")]
    PartnershipDuplicate,
    #[error("parent link already exists")]
    ParentLinkDuplicate,
    #[error("validation failed")]
    Validation(Vec<FieldViolation>),
    #[error("stale state")]
    ConflictStale,
    #[error("email {email} already in use")]
    EmailTaken { email: String },
    #[error("rate limited; retry after {retry_after_secs}s")]
    RateLimited { retry_after_secs: u32 },
    #[error("upstream {service}: {detail}")]
    Upstream { service: &'static str, detail: String },
    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}

impl ApiError {
    #[must_use]
    pub const fn code(&self) -> ErrorCode {
        match self {
            Self::Unauthenticated => ErrorCode::AuthUnauthenticated,
            Self::TokenExpired => ErrorCode::AuthTokenExpired,
            Self::TokenInvalid => ErrorCode::AuthTokenInvalid,
            Self::RefreshInvalid => ErrorCode::AuthRefreshInvalid,
            Self::MagicLinkInvalid => ErrorCode::AuthMagicLinkInvalid,
            Self::NotFamilyMember(_) => ErrorCode::FamilyNotMember,
            Self::InsufficientRole { .. } => ErrorCode::FamilyInsufficientRole,
            Self::InviteExpired => ErrorCode::FamilyInviteExpired,
            Self::PersonNotFound { .. } => ErrorCode::PersonNotFound,
            Self::PersonNotEditable => ErrorCode::PersonNotEditable,
            Self::ContactNotFound { .. } => ErrorCode::ContactNotFound,
            Self::ContactNotEditable => ErrorCode::ContactNotEditable,
            Self::ReminderNotFound => ErrorCode::ReminderNotFound,
            Self::RelationshipCycle => ErrorCode::RelationshipCycle,
            Self::PartnershipDuplicate => ErrorCode::PartnershipDuplicate,
            Self::ParentLinkDuplicate => ErrorCode::ParentLinkDuplicate,
            Self::Validation(_) => ErrorCode::ValidationFailed,
            Self::ConflictStale => ErrorCode::ConflictStale,
            Self::EmailTaken { .. } => ErrorCode::EmailTaken,
            Self::RateLimited { .. } => ErrorCode::RateLimited,
            Self::Upstream { .. } => ErrorCode::Upstream,
            Self::Internal(_) => ErrorCode::Internal,
        }
    }

    fn type_uri(code: ErrorCode) -> String {
        format!("https://my-family/errors/{}", code.slug())
    }

    /// Build the wire-safe body. Internal errors get a sanitized detail.
    #[must_use]
    pub fn to_body(&self, request_id: Option<&str>) -> ApiErrorBody {
        let code = self.code();
        let mut body = ApiErrorBody {
            type_: Self::type_uri(code),
            title: code.title(),
            status: code.http_status().as_u16(),
            code,
            detail: None,
            instance: request_id.map(str::to_string),
            fields: None,
        };
        match self {
            Self::Validation(violations) => {
                body.fields = Some(violations.clone());
            }
            Self::Internal(_) => {
                body.detail = Some(request_id.map_or_else(
                    || "Something went wrong. Please try again.".to_string(),
                    |rid| {
                        format!(
                            "Something went wrong. Please try again. If the problem persists, reference request id {rid}.",
                        )
                    },
                ));
            }
            _ => {
                body.detail = Some(self.to_string());
            }
        }
        body
    }
}

impl ResponseError for ApiError {
    fn status_code(&self) -> StatusCode {
        self.code().http_status()
    }

    fn error_response(&self) -> HttpResponse<BoxBody> {
        // Logging: internal at ERROR with full chain; rest at INFO/WARN.
        match self {
            Self::Internal(err) => {
                tracing::error!(error = ?err, code = ?self.code(), "internal error");
            }
            Self::RateLimited { retry_after_secs } => {
                tracing::warn!(retry_after_secs = retry_after_secs, code = ?self.code(), "rate limited");
            }
            Self::Validation(_) | Self::PersonNotFound { .. } | Self::ContactNotFound { .. } => {
                tracing::info!(code = ?self.code(), "client error");
            }
            _ => {
                tracing::warn!(code = ?self.code(), "client/auth error");
            }
        }

        let body = self.to_body(None);
        let mut resp = HttpResponse::build(self.status_code());
        resp.insert_header((CONTENT_TYPE, HeaderValue::from_static("application/problem+json")));
        if let Self::RateLimited { retry_after_secs } = self
            && let Ok(v) = HeaderValue::from_str(&retry_after_secs.to_string())
        {
            resp.insert_header((RETRY_AFTER, v));
        }
        resp.json(body)
    }
}

#[allow(clippy::result_large_err)]
pub type ApiResult<T> = Result<crate::ApiResponse<T>, ApiError>;

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use actix_web::body::to_bytes;

    use super::*;

    #[actix_web::test]
    async fn unauthenticated_maps_to_401_with_problem_json() {
        let err = ApiError::Unauthenticated;
        let resp = err.error_response();
        assert_eq!(resp.status(), 401);
        let ct = resp.headers().get(CONTENT_TYPE).unwrap().to_str().unwrap();
        assert_eq!(ct, "application/problem+json");
        let body = to_bytes(resp.into_body()).await.unwrap();
        let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(v["status"], 401);
        assert_eq!(v["code"], "auth_unauthenticated");
        assert_eq!(v["type"], "https://my-family/errors/auth.unauthenticated");
    }

    #[actix_web::test]
    async fn validation_includes_fields() {
        let err = ApiError::Validation(vec![FieldViolation::new(
            "/email",
            "validation.email_invalid",
            "must be an email",
        )]);
        let resp = err.error_response();
        assert_eq!(resp.status(), 422);
        let body = to_bytes(resp.into_body()).await.unwrap();
        let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(v["fields"][0]["path"], "/email");
    }

    #[actix_web::test]
    async fn rate_limited_adds_retry_after() {
        let err = ApiError::RateLimited { retry_after_secs: 42 };
        let resp = err.error_response();
        assert_eq!(resp.status(), 429);
        let ra = resp.headers().get("retry-after").unwrap().to_str().unwrap();
        assert_eq!(ra, "42");
    }

    #[actix_web::test]
    async fn internal_body_is_sanitized() {
        let err = ApiError::Internal(anyhow::anyhow!("database password 'hunter2' rejected"));
        let resp = err.error_response();
        assert_eq!(resp.status(), 500);
        let body = to_bytes(resp.into_body()).await.unwrap();
        let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let detail = v["detail"].as_str().unwrap();
        assert!(!detail.contains("hunter2"), "secret leaked into wire body");
        assert!(!detail.contains("password"), "secret leaked into wire body");
        assert_eq!(v["code"], "internal");
    }
}
