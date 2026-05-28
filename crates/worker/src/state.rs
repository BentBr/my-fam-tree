//! `WorkerState` — the worker's dependency-injection container.
//!
//! Handed to the ticker + dispatchers. Every collaborator is an `Arc<dyn …>`
//! so tests substitute fakes (in-memory queue, fake email) without touching
//! the loop logic.

use std::sync::Arc;

use my_family_cache::ReminderJobQueue;
use my_family_domain::{
    FamilyMembershipRepo, JanitorRepo, PartnershipRepo, PersonFavouriteRepo, PersonRepo,
    ReminderDigestRepo, ReminderPreferencesRepo, UserRepo,
};
use my_family_email::EmailSender;

use crate::clock::Clock;

#[derive(Clone)]
pub struct WorkerState {
    pub clock: Arc<dyn Clock>,
    pub users: Arc<dyn UserRepo>,
    pub memberships: Arc<dyn FamilyMembershipRepo>,
    pub persons: Arc<dyn PersonRepo>,
    pub partnerships: Arc<dyn PartnershipRepo>,
    pub favourites: Arc<dyn PersonFavouriteRepo>,
    pub prefs: Arc<dyn ReminderPreferencesRepo>,
    pub digests: Arc<dyn ReminderDigestRepo>,
    pub queue: Arc<dyn ReminderJobQueue>,
    pub email: Arc<dyn EmailSender>,
    pub janitor: Arc<dyn JanitorRepo>,
    pub web_public_url: String,
    pub max_retries: i32,
    pub retry_min_seconds: u64,
    pub retry_max_seconds: u64,
    /// Rows whose tombstone is younger than this are kept around (debug
    /// visibility); older rows get deleted on the next sweep.
    pub janitor_grace_seconds: u64,
}

impl std::fmt::Debug for WorkerState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WorkerState").finish_non_exhaustive()
    }
}
