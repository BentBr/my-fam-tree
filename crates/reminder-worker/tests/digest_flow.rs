//! Integration coverage for the reminder ticker + dispatcher against a real
//! Postgres (testcontainers). A fake in-memory queue + the email crate's
//! `FakeEmailSender` (and a deliberately-failing sender) stand in for Redis +
//! SMTP, and `FixedClock` pins "now" so the 06:00 firing window is
//! deterministic. CI runs these; they no-op-skip nowhere — they need Docker,
//! so they only run where a Docker socket is available (CI's backend-tests).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::future_not_send,
    clippy::indexing_slicing,
    reason = "test code: testcontainers + assertion helpers may panic and aren't Send-bounded"
)]

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use chrono::{NaiveDate, TimeZone, Utc};
use my_family_cache::{CacheError, ReminderJob, ReminderJobQueue};
use my_family_domain::{
    DigestStatus, FamilyMembershipRepo, FamilyRepo, Locale, PersonDraft, PersonRepo,
    ReminderPreferences, ReminderPreferencesRepo, Role, UserId, UserRepo,
};
use my_family_email::{EmailError, EmailSender, FakeEmailSender, OutboundEmail};
use my_family_persistence::{
    Database, PgFamilyMembershipRepo, PgFamilyRepo, PgPartnershipRepo, PgPersonFavouriteRepo,
    PgPersonRepo, PgReminderDigestRepo, PgReminderPrefsRepo, PgUserRepo,
};
use my_family_reminder_worker::clock::{Clock, FixedClock};
use my_family_reminder_worker::state::WorkerState;
use my_family_reminder_worker::{dispatcher, ticker};
use testcontainers::ContainerAsync;
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::postgres::Postgres;

// --- in-memory queue fake -------------------------------------------------

#[derive(Default)]
struct FakeQueue {
    jobs: Mutex<VecDeque<ReminderJob>>,
}

impl FakeQueue {
    fn len(&self) -> usize {
        self.jobs.lock().unwrap().len()
    }
}

#[async_trait]
impl ReminderJobQueue for FakeQueue {
    async fn push(&self, job: &ReminderJob) -> Result<(), CacheError> {
        self.jobs.lock().unwrap().push_back(job.clone());
        Ok(())
    }
    async fn try_pop(&self) -> Result<Option<ReminderJob>, CacheError> {
        Ok(self.jobs.lock().unwrap().pop_front())
    }
}

// --- always-failing email sender (retry path) -----------------------------

#[derive(Debug)]
struct FailingEmail;

#[async_trait]
impl EmailSender for FailingEmail {
    async fn send(&self, _email: OutboundEmail) -> Result<(), EmailError> {
        Err(EmailError::Smtp("simulated SMTP failure".into()))
    }
}

// --- harness --------------------------------------------------------------

async fn start_pg() -> (Database, ContainerAsync<Postgres>) {
    let pg = Postgres::default()
        .with_db_name("t")
        .with_user("t")
        .with_password("t")
        .start()
        .await
        .expect("start pg");
    let port = pg.get_host_port_ipv4(5432_u16).await.expect("pg port");
    let url = format!("postgres://t:t@127.0.0.1:{port}/t");
    let mut connected: Option<Database> = None;
    for _ in 0_u8..40_u8 {
        if let Ok(db) = Database::connect(&url, 4, Duration::from_secs(1), 30_000).await {
            connected = Some(db);
            break;
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
    let db = connected.expect("postgres never accepted connections");
    sqlx::migrate!("../../migrations").run(db.pool()).await.expect("migrate");
    (db, pg)
}

fn build_state(
    db: &Database,
    clock: Arc<dyn Clock>,
    queue: Arc<dyn ReminderJobQueue>,
    email: Arc<dyn EmailSender>,
) -> WorkerState {
    let pool = db.pool().clone();
    WorkerState {
        clock,
        users: Arc::new(PgUserRepo::new(pool.clone())),
        memberships: Arc::new(PgFamilyMembershipRepo::new(pool.clone())),
        persons: Arc::new(PgPersonRepo::new(pool.clone())),
        partnerships: Arc::new(PgPartnershipRepo::new(pool.clone())),
        favourites: Arc::new(PgPersonFavouriteRepo::new(pool.clone())),
        prefs: Arc::new(PgReminderPrefsRepo::new(pool.clone())),
        digests: Arc::new(PgReminderDigestRepo::new(pool)),
        queue,
        email,
        web_public_url: "https://app.example".into(),
        max_retries: 3,
        retry_min_seconds: 60,
        retry_max_seconds: 3_600,
    }
}

/// Seed a user (Europe/Berlin tz by default), a family they own, one person
/// with `birthday`, and the user's reminder preferences. Returns the user id.
async fn seed(
    db: &Database,
    email: &str,
    prefs: ReminderPreferences,
    birthday: NaiveDate,
) -> UserId {
    let pool = db.pool().clone();
    let users = PgUserRepo::new(pool.clone());
    let families = PgFamilyRepo::new(pool.clone());
    let memberships = PgFamilyMembershipRepo::new(pool.clone());
    let persons = PgPersonRepo::new(pool.clone());
    let prefs_repo = PgReminderPrefsRepo::new(pool);

    let user = users.create(email, Locale::En).await.expect("create user");
    let family = families.create("Fam", user.id).await.expect("create family");
    memberships.insert(family.id, user.id, Role::Owner).await.expect("insert membership");
    persons
        .create(
            family.id,
            PersonDraft {
                given_name: "Anna".into(),
                family_name: "Müller".into(),
                name_at_birth: String::new(),
                nickname: String::new(),
                gender: String::new(),
                birth_date: Some(birthday),
                birth_place: String::new(),
                death_date: None,
                notes: String::new(),
                linked_user_id: None,
            },
        )
        .await
        .expect("create person");
    prefs_repo.upsert(user.id, prefs).await.expect("upsert prefs");
    user.id
}

const fn enabled_prefs(lead_days: i32) -> ReminderPreferences {
    ReminderPreferences {
        emails_enabled: true,
        remind_birthdays: true,
        remind_anniversaries: true,
        favourites_only: false,
        lead_days,
    }
}

/// 06:00 Europe/Berlin on 2026-06-08 is 04:00 UTC (CEST = UTC+2). With
/// `lead_days` = 7 the target is 2026-06-15, so a 06-15 birthday fires.
fn berlin_0600_june_8() -> FixedClock {
    FixedClock::new(Utc.with_ymd_and_hms(2026, 6, 8, 4, 0, 0).single().unwrap())
}

const fn birthday_june_15() -> NaiveDate {
    NaiveDate::from_ymd_opt(1990, 6, 15).expect("valid date")
}

// --- tests ----------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn schedules_then_sends_digest_and_is_idempotent() {
    let (db, _pg) = start_pg().await;
    let user_id = seed(&db, "remind@example.com", enabled_prefs(7), birthday_june_15()).await;

    let queue = Arc::new(FakeQueue::default());
    let email = Arc::new(FakeEmailSender::new());
    let fixed = berlin_0600_june_8();
    let state = build_state(
        &db,
        Arc::new(fixed) as Arc<dyn Clock>,
        queue.clone() as Arc<dyn ReminderJobQueue>,
        email.clone() as Arc<dyn EmailSender>,
    );

    // First tick schedules + enqueues exactly one digest.
    assert_eq!(ticker::run_tick(&state).await.unwrap(), 1, "one digest scheduled");
    assert_eq!(queue.len(), 1, "one job enqueued");

    // Re-running the tick the same day is idempotent.
    assert_eq!(ticker::run_tick(&state).await.unwrap(), 0, "no duplicate scheduling");
    assert_eq!(queue.len(), 1, "no duplicate enqueue");

    // Dispatch the queued job → one email sent, digest marked sent.
    let job = ReminderJobQueue::try_pop(&*queue).await.unwrap().expect("a queued job");
    dispatcher::handle(&state, &job).await.unwrap();

    let sent = email.drain();
    assert_eq!(sent.len(), 1, "exactly one digest email");
    assert_eq!(sent[0].to_addr, "remind@example.com");
    assert!(sent[0].subject.contains('7'), "subject mentions the 7-day lead: {}", sent[0].subject);
    assert!(sent[0].text_body.contains("Anna"), "body lists the person: {}", sent[0].text_body);

    let digests = state.digests.list_for_user(user_id, 10).await.unwrap();
    assert_eq!(digests.len(), 1);
    assert_eq!(digests[0].status, DigestStatus::Sent);
    assert_eq!(digests[0].event_count, 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn retries_on_smtp_failure() {
    let (db, _pg) = start_pg().await;
    let user_id = seed(&db, "fail@example.com", enabled_prefs(7), birthday_june_15()).await;

    let queue = Arc::new(FakeQueue::default());
    let fixed = berlin_0600_june_8();
    let state = build_state(
        &db,
        Arc::new(fixed) as Arc<dyn Clock>,
        queue.clone() as Arc<dyn ReminderJobQueue>,
        Arc::new(FailingEmail) as Arc<dyn EmailSender>,
    );

    ticker::run_tick(&state).await.unwrap();
    let job = ReminderJobQueue::try_pop(&*queue).await.unwrap().expect("a queued job");
    dispatcher::handle(&state, &job).await.unwrap();

    // Send failed with attempts remaining ⇒ re-enqueued, still pending, attempt++.
    assert_eq!(queue.len(), 1, "re-enqueued for retry");
    let d = state.digests.list_for_user(user_id, 10).await.unwrap();
    assert_eq!(d[0].status, DigestStatus::Pending);
    assert_eq!(d[0].attempt_count, 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn no_digest_outside_the_0600_hour() {
    let (db, _pg) = start_pg().await;
    seed(&db, "off-hour@example.com", enabled_prefs(7), birthday_june_15()).await;

    let queue = Arc::new(FakeQueue::default());
    let email = Arc::new(FakeEmailSender::new());
    // 10:00 Europe/Berlin (08:00 UTC) — outside the firing window.
    let fixed = FixedClock::new(Utc.with_ymd_and_hms(2026, 6, 8, 8, 0, 0).single().unwrap());
    let state = build_state(
        &db,
        Arc::new(fixed) as Arc<dyn Clock>,
        queue.clone() as Arc<dyn ReminderJobQueue>,
        email as Arc<dyn EmailSender>,
    );

    assert_eq!(ticker::run_tick(&state).await.unwrap(), 0, "no scheduling outside 06:00");
    assert_eq!(queue.len(), 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn disabled_user_is_never_scheduled() {
    let (db, _pg) = start_pg().await;
    let mut prefs = enabled_prefs(7);
    prefs.emails_enabled = false;
    seed(&db, "disabled@example.com", prefs, birthday_june_15()).await;

    let queue = Arc::new(FakeQueue::default());
    let email = Arc::new(FakeEmailSender::new());
    let fixed = berlin_0600_june_8();
    let state = build_state(
        &db,
        Arc::new(fixed) as Arc<dyn Clock>,
        queue.clone() as Arc<dyn ReminderJobQueue>,
        email as Arc<dyn EmailSender>,
    );

    assert_eq!(ticker::run_tick(&state).await.unwrap(), 0, "emails-disabled user is skipped");
    assert_eq!(queue.len(), 0);
}
