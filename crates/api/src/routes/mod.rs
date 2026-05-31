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
pub mod person_favourites;
pub mod person_photos;
pub mod persons;
pub mod relationships;
pub mod reminder_prefs;
pub mod upcoming;
pub mod user_avatars;
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
        // `auth::logout` is mounted OUTSIDE the required-auth scope so
        // the FE can clear HttpOnly cookies on any "session is gone"
        // signal — including the case where the access cookie has
        // already expired and an auth-gated logout would 401. The
        // handler is idempotent: best-effort revoke the refresh row
        // keyed on the cookie (if present) and always emit
        // `Set-Cookie max-age=0` for both cookies. The response body
        // is the same fixed shape for every caller, so public access
        // reveals no session state.
        .service(auth::logout)
        // `invites::accept` is mounted OUTSIDE the required-auth scope:
        // the invite token itself is the auth factor. The handler
        // extracts JWT claims manually from the access cookie (if
        // present) so anonymous callers reach the find-or-create user
        // path while signed-in callers still get email-mismatch
        // validation.
        .service(invites::accept)
        .service(
            web::scope("")
                .wrap(AuthMiddleware::required())
                .service(auth::me)
                .service(users::me)
                .service(users::update)
                .service(users::email_change_request)
                .service(users::email_change_confirm)
                .service(user_avatars::upload)
                .service(user_avatars::clear)
                .service(families::list_mine)
                .service(families::create)
                .service(families::rename)
                .service(families::admin_overview)
                .service(families::delete_family)
                .service(families::invite)
                .service(invites::list_invites)
                .service(invites::cancel_invite)
                .service(persons::list)
                .service(persons::create)
                .service(persons::get_one)
                .service(persons::update)
                .service(persons::delete)
                .service(persons::claim)
                .service(person_favourites::set_favourite)
                .service(person_photos::upload)
                .service(person_photos::clear)
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
                .service(reminder_prefs::get_prefs)
                .service(reminder_prefs::put_prefs)
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
