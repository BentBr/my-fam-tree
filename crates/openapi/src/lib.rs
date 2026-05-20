//! Aggregates `utoipa::OpenApi` documents into a single spec.
//!
//! `utoipa` 5 cannot derive `ToSchema` for a bare generic, so each endpoint
//! declares a named `…ResponseBody` wrapper struct via the `response_body!`
//! macro in `crates/api/src/response.rs`. We import every wrapper and list it
//! in `components(schemas(...))`.

use utoipa::OpenApi;
use utoipa::openapi::security::{ApiKey, ApiKeyValue, SecurityScheme};

use my_family_api::{
    error::{ApiErrorBody, ErrorCode, FieldViolation},
    response::{NullResponseBody, Pagination, ResponseMeta},
    routes::health::{self, HealthResponseBody},
};

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
    paths(health::health),
    components(
        schemas(
            HealthResponseBody,
            NullResponseBody,
            ResponseMeta,
            Pagination,
            ApiErrorBody,
            ErrorCode,
            FieldViolation,
            health::Health,
        ),
    ),
    tags(
        (name = "health", description = "Liveness and readiness"),
    ),
)]
pub struct ApiDoc;

impl ApiDoc {
    /// Returns the `OpenAPI` doc with the `cookie_access` security scheme registered.
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
