//! HTTP API. Public from the binary entry point; openapi crate consumes the `ApiDoc`.

pub mod auth;
pub mod config;
pub mod cookies;
pub mod error;
pub mod middleware;
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
pub use config::{AppEnv, Config, ConfigError, LogFormat};
pub use error::{ApiError, ApiErrorBody, ApiResult, ErrorCode, FieldViolation};
pub use response::{ApiResponse, Pagination, ResponseMeta};
pub use state::AppState;
pub use tracing_setup::init_tracing;

/// Build the `Actix` `App` with the full middleware stack and route registration.
/// Shared by `bin/api.rs` and integration tests.
///
/// The returned `App` carries a deeply nested response-body type produced by
/// each middleware in the chain (`CORS` wraps in `EitherBody`, `TracingLogger`
/// in `StreamSpan`, `Logger` in `StreamLog`). We expose it through `impl
/// ServiceFactory` constrained only by `MessageBody` so callers don't need to
/// spell out the concrete nested type.
pub fn build_app(
    state: AppState,
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
    let allowed: Vec<&str> = cfg.cors_allowed_origins.split(',').map(str::trim).collect();
    // With `supports_credentials()`, the `CORS` spec forbids wildcards.
    // Enumerate the methods and headers we actually use.
    let mut cors = Cors::default()
        .allowed_methods(["GET", "POST", "PATCH", "DELETE", "OPTIONS"])
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
    App::new()
        .app_data(web::Data::new(state))
        .wrap(middleware::PanicCatcher)
        .wrap(actix_mw::Logger::new(
            r#"%a "%r" %s %b %T "%{Referer}i" "%{User-Agent}i" rid=%{x-request-id}o"#,
        ))
        .wrap(tracing_actix_web::TracingLogger::default())
        .wrap(middleware::RequestId)
        .wrap(cors)
        .service(routes::public_scope())
        .service(routes::auth_scope())
}
