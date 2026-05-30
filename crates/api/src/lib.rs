//! HTTP API. Public from the binary entry point; openapi crate consumes the `ApiDoc`.

pub mod auth;
pub mod cookies;
pub mod error;
pub mod images;
pub mod middleware;
pub mod multipart;
pub mod openapi_doc;
pub mod response;
pub mod routes;
pub mod services;
pub mod state;
pub mod tracing_setup;
pub mod validation;

use actix_cors::Cors;
use actix_web::body::MessageBody;
use actix_web::http::header::HeaderName;
use actix_web::{App, middleware as actix_mw, web};
pub use error::{ApiError, ApiErrorBody, ApiResult, ErrorCode, FieldViolation};
pub use my_fam_tree_config::{ApiConfig as Config, AppEnv, ConfigError, LogFormat};
pub use openapi_doc::ApiDoc;
pub use response::{ApiResponse, Pagination, ResponseMeta, Warning};
pub use state::AppState;
pub use tracing_setup::init_tracing;
use utoipa_swagger_ui::SwaggerUi;

/// Build the `Actix` `App` with the full middleware stack and route registration.
/// Shared by `bin/api.rs` and integration tests.
///
/// The returned `App` carries a deeply nested response-body type produced by
/// each middleware in the chain (`CORS` wraps in `EitherBody`, `TracingLogger`
/// in `StreamSpan`, `Logger` in `StreamLog`). We expose it through `impl
/// ServiceFactory` constrained only by `MessageBody` so callers don't need to
/// spell out the concrete nested type.
///
/// `openapi` is `Some(spec)` when the caller wants Swagger UI mounted under
/// `/api/docs/`; tests pass `None`. Even with `Some`, the UI is only mounted
/// when `state.cfg.api.enable_docs` is `true`.
pub fn build_app(
    state: AppState,
    openapi: Option<utoipa::openapi::OpenApi>,
) -> App<
    impl actix_web::dev::ServiceFactory<
        actix_web::dev::ServiceRequest,
        Config = (),
        Response = actix_web::dev::ServiceResponse<impl MessageBody + use<>>,
        Error = actix_web::Error,
        InitError = (),
    > + use<>,
> {
    let cfg = state.cfg.clone();
    let api_enable_docs = cfg.api.enable_docs;
    let allowed: Vec<&str> = cfg.api.cors_allowed_origins.split(',').map(str::trim).collect();
    // With `supports_credentials()`, the `CORS` spec forbids wildcards.
    // Enumerate the methods and headers we actually use.
    //
    // PUT is required for `/reminder-preferences` (full-replace upsert
    // semantics — the only PUT in the surface today). Omitting it makes
    // the browser's preflight reject the request before it leaves the
    // SPA, surfacing as a "CORS error" with no useful detail. Keep the
    // list in sync if a new HTTP method appears in `routes/`.
    let mut cors = Cors::default()
        .allowed_methods(["GET", "POST", "PUT", "PATCH", "DELETE", "OPTIONS"])
        .allowed_headers([
            actix_web::http::header::CONTENT_TYPE,
            actix_web::http::header::ACCEPT,
            actix_web::http::header::ACCEPT_LANGUAGE,
            HeaderName::from_static("x-family-id"),
            HeaderName::from_static("x-request-id"),
        ])
        .expose_headers([HeaderName::from_static("x-request-id")])
        .supports_credentials()
        .max_age(3600);
    for origin in allowed {
        cors = cors.allowed_origin(origin);
    }

    // Actix wraps in reverse order: the outermost `.wrap(...)` runs LAST around
    // the chain. To get the logical order
    //   request -> CORS -> RequestId -> TracingLogger -> AccessLog -> PanicCatcher -> handler
    // we register them in reverse here.
    //
    // The Swagger UI service is registered via `.configure(...)` so we can
    // conditionally mount it without exploding the `App`'s type parameter.
    // `ServiceConfig::service` accepts any `HttpServiceFactory`.
    App::new()
        .app_data(web::Data::new(state))
        .wrap(middleware::PanicCatcher)
        .wrap(actix_mw::Logger::new(
            r#"%a "%r" %s %b %T "%{Referer}i" "%{User-Agent}i" rid=%{x-request-id}o"#,
        ))
        .wrap(tracing_actix_web::TracingLogger::default())
        .wrap(middleware::RequestId)
        .wrap(cors)
        .service(routes::api_scope())
        .configure(move |app_cfg| {
            if api_enable_docs && let Some(doc) = openapi {
                app_cfg
                    .service(SwaggerUi::new("/api/docs/{_:.*}").url("/api/docs/openapi.json", doc));
            }
        })
}
