//! Route registration.
//!
//! A single `/api/v1` scope hosts the entire HTTP surface. Public endpoints
//! (`/health`, the unauthenticated half of the auth flow) mount directly at
//! the top level; auth-required endpoints (`/auth/logout`, `/auth/me`,
//! `/families/*`, `/invites/accept`) live inside a nested empty-path scope
//! that wraps [`AuthMiddleware::required`].
//!
//! Mounting two separate `web::scope("/api/v1")` services at the `App` level
//! shadows the second one: actix's resource tree picks the first matching
//! prefix and returns `404` for paths it doesn't define rather than falling
//! through. The nested-empty-scope pattern avoids that footgun while keeping
//! the auth middleware scoped to exactly the routes that need it.

pub mod audit;
pub mod auth;
pub mod contacts;
pub mod families;
pub mod health;
pub mod invites;
pub mod members;
pub mod owner_transfer;
pub mod parent_links;
pub mod partnerships;
pub mod persons;
pub mod relationships;
pub mod upcoming;
pub mod users;

use actix_web::web;

use crate::auth::AuthMiddleware;

#[must_use]
pub fn api_scope() -> actix_web::Scope<
    impl actix_web::dev::ServiceFactory<
        actix_web::dev::ServiceRequest,
        Config = (),
        Response = actix_web::dev::ServiceResponse<actix_web::body::BoxBody>,
        Error = actix_web::Error,
        InitError = (),
    > + use<>,
> {
    web::scope("/api/v1")
        .service(health::health)
        .service(auth::magic_link)
        .service(auth::consume)
        .service(auth::refresh)
        .service(
            web::scope("")
                .wrap(AuthMiddleware::required())
                .service(auth::logout)
                .service(auth::me)
                .service(users::me)
                .service(users::update)
                .service(users::email_change_request)
                .service(users::email_change_confirm)
                .service(families::list_mine)
                .service(families::create)
                .service(families::rename)
                .service(families::delete_family)
                .service(families::invite)
                .service(invites::accept)
                .service(invites::list_invites)
                .service(invites::cancel_invite)
                .service(persons::list)
                .service(persons::create)
                .service(persons::get_one)
                .service(persons::update)
                .service(persons::delete)
                .service(parent_links::create)
                .service(parent_links::delete)
                .service(partnerships::create)
                .service(partnerships::update)
                .service(partnerships::delete)
                .service(contacts::list_for_person)
                .service(contacts::create)
                .service(contacts::update)
                .service(contacts::delete)
                .service(relationships::tree)
                .service(upcoming::list)
                .service(audit::list_audit)
                .service(members::list_members)
                .service(members::set_member_role)
                .service(members::revoke_member)
                .service(owner_transfer::begin)
                .service(owner_transfer::confirm)
                .service(owner_transfer::cancel)
                .service(owner_transfer::status),
        )
}
