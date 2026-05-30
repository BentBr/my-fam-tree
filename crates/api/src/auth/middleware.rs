//! Auth middleware: verify JWT cookie + resolve `X-Family-Id`.
//!
//! On each request:
//!   1. Read the access cookie. Missing or invalid -> see "required" below.
//!   2. Verify the JWT via the `AppState` issuer (signature, `iss`, `aud`, `exp`).
//!   3. Parse `X-Family-Id` (if present) and cross-reference it against
//!      `claims.families`. A header value that does not match any membership is
//!      silently dropped — handlers that need an active family must call
//!      [`user_claims_with_family`] to surface the missing-header validation
//!      error explicitly.
//!   4. Insert [`UserClaims`] into the request extensions for handlers.
//!
//! `required()` middleware returns `401` when no valid cookie is present.
//! `optional()` middleware passes through; handlers must guard their own use of
//! the extension. The two flavours are kept symmetrical so the routing scopes
//! in `routes::mod` can compose either.

use std::rc::Rc;
use std::sync::LazyLock;

use actix_web::ResponseError;
use actix_web::body::{BoxBody, MessageBody};
use actix_web::dev::{Service, ServiceRequest, ServiceResponse, Transform, forward_ready};
use actix_web::http::header::HeaderName;
use actix_web::{Error, HttpMessage, HttpRequest, web};
use futures_util::future::{LocalBoxFuture, Ready, ready};
use my_fam_tree_domain::{FamilyId, Role, UserId};
use uuid::Uuid;

use crate::auth::user_claims::{ActiveFamily, FamilyMembershipMirror, UserClaims};
use crate::cookies::ACCESS_COOKIE;
use crate::{ApiError, AppState, FieldViolation};

/// `x-family-id` request header. `HeaderName::from_static` is not const-fn in
/// `http` 1.x, so we go through `LazyLock`, mirroring `middleware::request_id`.
pub static FAMILY_HEADER: LazyLock<HeaderName> =
    LazyLock::new(|| HeaderName::from_static("x-family-id"));

/// `Transform` factory.
///
/// If `required`, requests without a valid access cookie are rejected as
/// `ApiError::Unauthenticated`. If `optional`, the handler is reached with no
/// `UserClaims` extension and must decide what to do.
#[derive(Debug, Clone, Copy)]
pub struct AuthMiddleware {
    pub required: bool,
}

impl AuthMiddleware {
    #[must_use]
    pub const fn required() -> Self {
        Self { required: true }
    }
    #[must_use]
    pub const fn optional() -> Self {
        Self { required: false }
    }
}

#[derive(Debug)]
pub struct AuthService<S> {
    service: Rc<S>,
    required: bool,
}

impl<S, B> Transform<S, ServiceRequest> for AuthMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    B: MessageBody + 'static,
{
    type Response = ServiceResponse<BoxBody>;
    type Error = Error;
    type Transform = AuthService<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(AuthService { service: Rc::new(service), required: self.required }))
    }
}

impl<S, B> Service<ServiceRequest> for AuthService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    B: MessageBody + 'static,
{
    type Response = ServiceResponse<BoxBody>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let svc = Rc::clone(&self.service);
        let required = self.required;
        Box::pin(async move {
            let state = req.app_data::<web::Data<AppState>>().cloned();
            let claims_opt = state.and_then(|s| extract_claims(&req, &s.jwt_issuer));

            match (claims_opt, required) {
                (Some((claims, header_family)), _) => {
                    let user_claims = build_user_claims(claims, header_family);
                    req.extensions_mut().insert(user_claims);
                    svc.call(req).await.map(ServiceResponse::map_into_boxed_body)
                }
                (None, true) => {
                    // CRITICAL: do NOT `Err(Error::from(ApiError::Unauthenticated))`
                    // here. `actix-cors`'s response-decoration path runs AFTER
                    // its `let res = fut.await?` line, so an `Err` propagates
                    // up the chain WITHOUT going through CORS's header
                    // injection. The browser then receives the 401 with no
                    // `Access-Control-Allow-Origin` and reports it as a CORS
                    // failure — masking the real 401 and breaking cross-origin
                    // auth probes from the SPA.
                    //
                    // Synthesising an `Ok(ServiceResponse)` keeps the 401 body
                    // identical (same `ApiError::Unauthenticated.error_response()`,
                    // same `application/problem+json` shape) but lets CORS see
                    // it as a normal response and decorate it. Same body, same
                    // status, correct headers.
                    let resp = ApiError::Unauthenticated.error_response();
                    Ok(req.into_response(resp))
                }
                (None, false) => svc.call(req).await.map(ServiceResponse::map_into_boxed_body),
            }
        })
    }
}

fn extract_claims(
    req: &ServiceRequest,
    issuer: &crate::auth::JwtIssuer,
) -> Option<(crate::auth::JwtClaims, Option<Uuid>)> {
    let cookie_value = req.cookie(ACCESS_COOKIE).map(|c| c.value().to_string())?;
    let claims = issuer.verify(&cookie_value).ok()?;
    let header_family = req
        .headers()
        .get(&*FAMILY_HEADER)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| Uuid::parse_str(s).ok());
    Some((claims, header_family))
}

fn build_user_claims(claims: crate::auth::JwtClaims, header_family: Option<Uuid>) -> UserClaims {
    let all_families: Vec<FamilyMembershipMirror> = claims
        .families
        .iter()
        .map(|f| FamilyMembershipMirror {
            id: FamilyId::from_uuid(f.id),
            name: f.name.clone(),
            role: f.role,
        })
        .collect();

    let active = header_family.and_then(|id| {
        claims.families.iter().find(|f| f.id == id).map(|f| ActiveFamily {
            id: FamilyId::from_uuid(f.id),
            name: f.name.clone(),
            role: f.role,
        })
    });

    UserClaims {
        user_id: UserId::from_uuid(claims.sub),
        email: claims.email,
        locale: claims.locale,
        active_family: active,
        all_families,
    }
}

/// Extract `UserClaims` from request extensions. Assumes the `required` flavour
/// of the middleware ran; otherwise returns `Unauthenticated`.
///
/// # Errors
/// Returns [`ApiError::Unauthenticated`] when the extension is absent.
pub fn user_claims(req: &HttpRequest) -> Result<UserClaims, ApiError> {
    req.extensions().get::<UserClaims>().cloned().ok_or(ApiError::Unauthenticated)
}

/// Extract `UserClaims` if present without erroring. Used by routes
/// that accept anonymous callers (e.g. `/invites/accept` where the
/// invite token itself is the auth factor).
#[must_use]
pub fn try_user_claims(req: &HttpRequest) -> Option<UserClaims> {
    req.extensions().get::<UserClaims>().cloned()
}

/// Extract `UserClaims` and require an active family (a valid `X-Family-Id`
/// header that resolved against the JWT memberships).
///
/// # Errors
/// Returns [`ApiError::Unauthenticated`] when no claims are present, or
/// [`ApiError::Validation`] when the `X-Family-Id` header is missing or did
/// not match any membership in the access token.
pub fn user_claims_with_family(req: &HttpRequest) -> Result<(UserClaims, ActiveFamily), ApiError> {
    let claims = user_claims(req)?;
    let active = claims.active_family.clone().ok_or_else(|| {
        ApiError::Validation(vec![family_header_required("X-Family-Id required")])
    })?;
    Ok((claims, active))
}

fn family_header_required(msg: &str) -> FieldViolation {
    FieldViolation::new("/headers/x-family-id", "validation.header_required", msg)
        .with_param("header", "X-Family-Id")
}

/// Assert the active family's role meets `needed`.
///
/// JWT-only check — fast, but trusts whatever role the access cookie
/// was issued with. For authz-sensitive WRITES (member role changes,
/// family rename/delete/invite, owner transfer) prefer
/// [`require_db_role`] so a freshly-demoted user can't keep using their
/// stale access cookie until it expires.
///
/// # Errors
/// Returns [`ApiError::InsufficientRole`] when the active role is below `needed`.
pub const fn require_role(active: &ActiveFamily, needed: Role) -> Result<(), ApiError> {
    if active.role.at_least(needed) { Ok(()) } else { Err(ApiError::InsufficientRole { needed }) }
}

/// Assert the role recorded in the DB right now is at least `needed`,
/// not the role baked into the access JWT.
///
/// The JWT memberships claim is a snapshot from issuance time. An admin
/// who was demoted to `user` (or revoked) keeps the old claim until the
/// access cookie expires — that's the "stale privilege window" the
/// security review called out. Authz-sensitive WRITES route through
/// here so a demoted user's next privileged mutation gets the current
/// DB state, not the cached JWT.
///
/// One extra DB round-trip per call site, acceptable for the write
/// paths it guards (these are user-perceived single-action flows, not
/// hot loops).
///
/// # Errors
/// * [`ApiError::NotFamilyMember`] when no membership row exists.
/// * [`ApiError::InsufficientRole`] when the row's role is below `needed`.
/// * [`ApiError::Internal`] for DB transport / driver failures.
pub async fn require_db_role(
    state: &crate::AppState,
    user_id: my_fam_tree_domain::UserId,
    family_id: my_fam_tree_domain::FamilyId,
    needed: Role,
) -> Result<(), ApiError> {
    let membership = state
        .memberships
        .find(family_id, user_id)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!(e.to_string())))?;
    match membership {
        None => Err(ApiError::NotFamilyMember(family_id.into_uuid())),
        Some(m) if !m.role.at_least(needed) => Err(ApiError::InsufficientRole { needed }),
        Some(_) => Ok(()),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    // Importing `actix_web::test` unqualified shadows the std `#[test]`
    // attribute for the rest of this module; alias it to keep the existing
    // sync `#[test]` fixtures compiling.
    use actix_web::test as actix_test;
    use actix_web::{App, HttpResponse, web};
    use my_fam_tree_domain::Role;
    use uuid::Uuid;

    use super::*;
    use crate::auth::claims::{FamilyClaim, JwtClaims};

    async fn protected() -> HttpResponse {
        HttpResponse::Ok().body("ok")
    }

    /// Regression: a missing-cookie request to a guarded route MUST resolve
    /// to an `Ok(ServiceResponse)` with status 401, not `Err`. The previous
    /// `Err(Error::from(ApiError::Unauthenticated))` shape short-circuited
    /// `actix-cors`'s response-decoration path (its `let res = fut.await?`),
    /// so the 401 reached the browser without `Access-Control-Allow-Origin`
    /// and surfaced as a misleading "CORS error" instead of the real 401.
    /// Asserting `Ok` here pins the contract — if a future refactor reverts
    /// to `Err`, this test fails before the SPA does.
    #[actix_web::test]
    async fn missing_cookie_returns_ok_401_so_cors_can_decorate() {
        let app = actix_test::init_service(
            App::new().service(
                web::scope("")
                    .wrap(AuthMiddleware::required())
                    .route("/p", web::get().to(protected)),
            ),
        )
        .await;
        let req = actix_test::TestRequest::get().uri("/p").to_request();
        // `try_call_service` returns `Err` only when the inner Service errors;
        // an `Ok` with a 401 status passes through as `Ok`. The bug we're
        // guarding against would return `Err` here.
        let resp = actix_test::try_call_service(&app, req)
            .await
            .expect("auth middleware must return Ok(401), not Err — CORS would skip the response");
        assert_eq!(resp.status(), 401);
    }

    fn fixture_claims(families: Vec<FamilyClaim>) -> JwtClaims {
        JwtClaims {
            iss: "iss".into(),
            aud: "aud".into(),
            sub: Uuid::new_v4(),
            email: "a@b.c".into(),
            locale: "en".into(),
            families,
            iat: 0,
            exp: 0,
            jti: "j".into(),
        }
    }

    #[test]
    fn build_user_claims_without_header_has_no_active_family() {
        let fam_id = Uuid::new_v4();
        let claims = fixture_claims(vec![FamilyClaim {
            id: fam_id,
            name: "Müller".into(),
            role: Role::Owner,
        }]);
        let uc = build_user_claims(claims, None);
        assert_eq!(uc.email, "a@b.c");
        assert_eq!(uc.all_families.len(), 1);
        assert_eq!(uc.all_families[0].id.into_uuid(), fam_id);
        assert!(uc.active_family.is_none());
    }

    #[test]
    fn build_user_claims_with_matching_header_resolves_active_family() {
        let fam_id = Uuid::new_v4();
        let claims = fixture_claims(vec![FamilyClaim {
            id: fam_id,
            name: "Müller".into(),
            role: Role::Admin,
        }]);
        let uc = build_user_claims(claims, Some(fam_id));
        let active = uc.active_family.expect("active family resolved");
        assert_eq!(active.id.into_uuid(), fam_id);
        assert_eq!(active.role, Role::Admin);
    }

    #[test]
    fn build_user_claims_with_unknown_header_drops_active_family() {
        let claims = fixture_claims(vec![FamilyClaim {
            id: Uuid::new_v4(),
            name: "Müller".into(),
            role: Role::User,
        }]);
        let uc = build_user_claims(claims, Some(Uuid::new_v4()));
        assert!(uc.active_family.is_none());
    }

    #[test]
    fn require_role_admin_blocks_user_allows_admin_and_owner() {
        let mk =
            |role| ActiveFamily { id: FamilyId::from_uuid(Uuid::new_v4()), name: "f".into(), role };
        assert!(matches!(
            require_role(&mk(Role::User), Role::Admin),
            Err(ApiError::InsufficientRole { needed: Role::Admin })
        ));
        assert!(require_role(&mk(Role::Admin), Role::Admin).is_ok());
        assert!(require_role(&mk(Role::Owner), Role::Admin).is_ok());
    }

    #[test]
    fn family_header_required_violation_uses_stable_path_and_code() {
        let v = family_header_required("X-Family-Id required");
        assert_eq!(v.path, "/headers/x-family-id");
        assert_eq!(v.code, "validation.header_required");
        assert_eq!(v.message, "X-Family-Id required");
    }
}
