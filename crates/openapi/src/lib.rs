//! Aggregates `utoipa::OpenApi` documents into a single spec.
//!
//! `utoipa` 5 cannot derive `ToSchema` for a bare generic, so each endpoint
//! declares a named `…ResponseBody` wrapper struct via the `response_body!`
//! macro in `crates/api/src/response.rs`. We import every wrapper and list it
//! in `components(schemas(...))`. The runtime handlers continue to return
//! `ApiResponse<T>`; the wrappers are schema-only.

use my_family_api::auth::{FamilyClaim, JwtClaims};
use my_family_api::error::{ApiErrorBody, ErrorCode, FieldViolation};
use my_family_api::response::{NullResponseBody, Pagination, ResponseMeta};
use my_family_api::routes::auth::{
    self, ConsumeResponseBody, LogoutResponseBody, MagicLinkResponseBody, MeResponseBody,
};
use my_family_api::routes::families::{
    self, CreateFamilyResponseBody, FamilyViewResponseBody, InviteResponseBody,
    MyFamiliesResponseBody,
};
use my_family_api::routes::health::{self, HealthResponseBody};
use my_family_api::routes::invites::{self, AcceptResponseBody};
use utoipa::OpenApi;
use utoipa::openapi::security::{ApiKey, ApiKeyValue, SecurityScheme};

/// Aggregated `OpenAPI` document for the entire HTTP surface.
#[derive(Debug, OpenApi)]
#[openapi(
    info(
        title = "my-family API",
        description = "Family platform API",
        version = env!("CARGO_PKG_VERSION"),
    ),
    servers(
        (url = "/", description = "Same-origin"),
    ),
    paths(
        health::health,
        auth::magic_link,
        auth::consume,
        auth::refresh,
        auth::logout,
        auth::me,
        families::list_mine,
        families::create,
        families::rename,
        families::delete_family,
        families::invite,
        invites::accept,
    ),
    components(
        schemas(
            // Named envelope wrappers — utoipa cannot derive ToSchema for
            // generic ApiResponse<T>. Each handler declares its own wrapper
            // via the `response_body!` macro in `crates/api/src/response.rs`.
            HealthResponseBody,
            MagicLinkResponseBody,
            ConsumeResponseBody,
            LogoutResponseBody,
            MeResponseBody,
            MyFamiliesResponseBody,
            CreateFamilyResponseBody,
            FamilyViewResponseBody,
            InviteResponseBody,
            AcceptResponseBody,
            // Shared wrapper for DELETE / void-data responses.
            NullResponseBody,
            // Envelope + error scalars (shared across every response).
            ResponseMeta,
            Pagination,
            ApiErrorBody,
            ErrorCode,
            FieldViolation,
            // Concrete payload types.
            health::Health,
            FamilyClaim,
            JwtClaims,
            auth::MagicLinkReq,
            auth::ConsumeReq,
            auth::MagicLinkRes,
            auth::ConsumeRes,
            auth::LogoutRes,
            families::MyFamiliesRes,
            families::FamilyView,
            families::CreateFamilyReq,
            families::CreateFamilyRes,
            families::RenameFamilyReq,
            families::InviteReq,
            families::InviteRes,
            invites::AcceptReq,
            invites::AcceptRes,
        ),
    ),
    tags(
        (name = "health", description = "Liveness and readiness"),
        (name = "auth", description = "Authentication"),
        (name = "families", description = "Family management"),
        (name = "invites", description = "Invite acceptance"),
    ),
)]
pub struct ApiDoc;

impl ApiDoc {
    /// Returns the `OpenAPI` doc with the `cookie_access` security scheme registered.
    /// The FE talks to the API via `HttpOnly` cookies, not HTTP Bearer; this
    /// scheme mirrors the actual transport so generated clients describe it
    /// correctly.
    #[must_use]
    pub fn with_cookie_auth() -> utoipa::openapi::OpenApi {
        let mut doc = Self::openapi();
        let mut components = doc.components.unwrap_or_default();
        components.add_security_scheme(
            "cookie_access",
            SecurityScheme::ApiKey(ApiKey::Cookie(ApiKeyValue::new("access"))),
        );
        doc.components = Some(components);
        doc
    }
}
