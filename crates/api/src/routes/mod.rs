//! Route registration.
//!
//! Two scopes mount under `/api/v1`:
//!
//! - [`public_scope`] — endpoints reachable without a session: `/health` and
//!   the unauthenticated half of the auth flow (`/auth/magic-link`,
//!   `/auth/consume`, `/auth/refresh`).
//! - [`auth_scope`] — wrapped in [`AuthMiddleware::required`] so handlers can
//!   trust that `crate::auth::user_claims` returns a verified session
//!   (`/auth/logout`, `/auth/me`, `/families/*`, `/invites/accept`).

pub mod auth;
pub mod families;
pub mod health;
pub mod invites;

use actix_web::web;

use crate::auth::AuthMiddleware;

#[must_use]
pub fn public_scope() -> actix_web::Scope {
    web::scope("/api/v1")
        .service(health::health)
        .service(auth::magic_link)
        .service(auth::consume)
        .service(auth::refresh)
}

#[must_use]
pub fn auth_scope() -> actix_web::Scope<
    impl actix_web::dev::ServiceFactory<
        actix_web::dev::ServiceRequest,
        Config = (),
        Response = actix_web::dev::ServiceResponse<actix_web::body::BoxBody>,
        Error = actix_web::Error,
        InitError = (),
    > + use<>,
> {
    web::scope("/api/v1")
        .wrap(AuthMiddleware::required())
        .service(auth::logout)
        .service(auth::me)
        .service(families::list_mine)
        .service(families::create)
        .service(families::rename)
        .service(families::delete_family)
        .service(families::invite)
        .service(invites::accept)
}
