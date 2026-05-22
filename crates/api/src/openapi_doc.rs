//! Aggregated `utoipa::OpenApi` document for the entire HTTP surface.
//!
//! Lives in the api crate (not the openapi crate) so [`build_app`] can mount
//! `utoipa-swagger-ui` against the same spec without introducing a circular
//! dependency. The openapi crate's `openapi-dump` binary re-exports the type
//! through `my_family_openapi::ApiDoc` for the CI / FE codegen pipeline.
//!
//! `utoipa` 5 cannot derive `ToSchema` for a bare generic, so each endpoint
//! declares a named `…ResponseBody` wrapper struct via the `response_body!`
//! macro in `crates/api/src/response.rs`. We import every wrapper and list it
//! in `components(schemas(...))`. The runtime handlers continue to return
//! `ApiResponse<T>`; the wrappers are schema-only.
//!
//! [`build_app`]: crate::build_app

use utoipa::OpenApi;
use utoipa::openapi::security::{ApiKey, ApiKeyValue, SecurityScheme};

use crate::auth::{FamilyClaim, JwtClaims};
use crate::error::{ApiErrorBody, ErrorCode, FieldViolation};
use crate::response::{NullResponseBody, Pagination, ResponseMeta, Warning};
use crate::routes::auth::{
    self, ConsumeResponseBody, LogoutResponseBody, MagicLinkResponseBody, MeResponseBody,
};
use crate::routes::families::{
    self, CreateFamilyResponseBody, FamilyViewResponseBody, InviteResponseBody,
    MyFamiliesResponseBody,
};
use crate::routes::health::{self, HealthResponseBody};
use crate::routes::invites::{self, AcceptResponseBody};
use crate::routes::parent_links;
use crate::routes::partnerships::{self, PartnershipViewResponseBody};
use crate::routes::persons::{self, PersonViewResponseBody, PersonsListResponseBody};
use crate::routes::relationships::{self, TreePayloadResponseBody};
use crate::routes::users::{self, EmailChangeResponseBody, UserProfileResponseBody};
use crate::services::relationships_tree::{EdgePair, TreeNode, TreePayload};

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
        users::me,
        users::update,
        users::email_change_request,
        users::email_change_confirm,
        persons::list,
        persons::create,
        persons::get_one,
        persons::update,
        persons::delete,
        parent_links::create,
        parent_links::delete,
        partnerships::create,
        partnerships::update,
        partnerships::delete,
        relationships::tree,
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
            UserProfileResponseBody,
            EmailChangeResponseBody,
            PersonsListResponseBody,
            PersonViewResponseBody,
            PartnershipViewResponseBody,
            TreePayloadResponseBody,
            // Shared wrapper for DELETE / void-data responses.
            NullResponseBody,
            // Envelope + error scalars (shared across every response).
            ResponseMeta,
            Pagination,
            Warning,
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
            users::UserProfile,
            users::UpdateUserReq,
            users::EmailChangeReq,
            users::EmailChangeRes,
            users::EmailChangeConfirmReq,
            persons::PersonView,
            persons::PersonCreateReq,
            persons::PersonUpdateReq,
            persons::PersonsQuery,
            parent_links::ParentLinkReq,
            partnerships::PartnershipView,
            partnerships::PartnershipCreateReq,
            partnerships::PartnershipUpdateReq,
            TreePayload,
            TreeNode,
            EdgePair,
        ),
    ),
    tags(
        (name = "health", description = "Liveness and readiness"),
        (name = "auth", description = "Authentication"),
        (name = "families", description = "Family management"),
        (name = "invites", description = "Invite acceptance"),
        (name = "users", description = "User profile and email change"),
        (name = "persons", description = "Family members"),
        (name = "relationships", description = "Parent links, partnerships, tree"),
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
