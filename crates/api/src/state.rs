//! `AppState` — the single dependency-injection container.
//!
//! Handed to every handler via `web::Data<AppState>`. Each collaborator is stored as an
//! `Arc<dyn …>` so production wiring lives in `bin/api.rs` and integration tests can
//! substitute fakes without touching handler code.
//!
//! The `Debug` impl is intentionally opaque: several inner collaborators hold key
//! material, connection pools, or wrap raw transports we never want leaked into logs.
//! `missing_debug_implementations` is enforced workspace-wide, hence the manual impl.

use std::sync::Arc;

use my_family_cache::{RateLimiter, RedisPool};
use my_family_domain::{
    AuditLogRepo, FamilyInviteRepo, FamilyMembershipRepo, FamilyRepo, MagicLinkRepo,
    ParentLinkRepo, PartnershipRepo, PersonContactRepo, PersonRepo, RefreshTokenRepo, UserRepo,
};
use my_family_email::EmailSender;

use crate::Config;
use crate::auth::JwtIssuer;

#[derive(Clone)]
pub struct AppState {
    pub cfg: Arc<Config>,
    pub users: Arc<dyn UserRepo>,
    pub magic_links: Arc<dyn MagicLinkRepo>,
    pub refresh_tokens: Arc<dyn RefreshTokenRepo>,
    pub families: Arc<dyn FamilyRepo>,
    pub memberships: Arc<dyn FamilyMembershipRepo>,
    pub invites: Arc<dyn FamilyInviteRepo>,
    pub persons: Arc<dyn PersonRepo>,
    pub parent_links: Arc<dyn ParentLinkRepo>,
    pub partnerships: Arc<dyn PartnershipRepo>,
    pub contacts: Arc<dyn PersonContactRepo>,
    pub audit: Arc<dyn AuditLogRepo>,
    pub email: Arc<dyn EmailSender>,
    pub rate_limiter: Arc<dyn RateLimiter>,
    pub redis: RedisPool,
    pub jwt_issuer: Arc<JwtIssuer>,
}

impl std::fmt::Debug for AppState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppState").finish_non_exhaustive()
    }
}
