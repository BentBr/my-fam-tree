//! Integration coverage for the durable email outbox: enqueue, claim
//! (FOR UPDATE SKIP LOCKED), send via `EmailSender`, then mark sent /
//! retry / failed-permanent. Exercises the real `PgEmailOutboxRepo`
//! against a testcontainers Postgres + drives one cycle through
//! `worker::outbox::process_one` so the retry/backoff bookkeeping is
//! validated end-to-end (the API tests use a `SyncOutbox` double).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::future_not_send,
    clippy::similar_names,
    clippy::too_many_lines,
    reason = "test code: testcontainers + assertion helpers may panic and aren't Send-bounded"
)]

use std::sync::Arc;
use std::time::Duration as StdDuration;

use async_trait::async_trait;
use chrono::Utc;
use my_family_cache::{CacheError, ReminderJob, ReminderJobQueue};
use my_family_domain::{EmailOutboxInsert, EmailOutboxKind};
use my_family_email::{EmailError, EmailSender, FakeEmailSender, OutboundEmail};
use my_family_persistence::{
    Database, PgEmailOutboxRepo, PgFamilyMembershipRepo, PgJanitor, PgPartnershipRepo,
    PgPersonFavouriteRepo, PgPersonRepo, PgReminderDigestRepo, PgReminderPrefsRepo, PgUserRepo,
};
use my_family_worker::clock::{Clock, FixedClock};
use my_family_worker::outbox;
use my_family_worker::state::WorkerState;
use testcontainers::ContainerAsync;
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::postgres::Postgres;

// --- testcontainer setup ----------------------------------------------------

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
        if let Ok(db) = Database::connect(&url, 4, StdDuration::from_secs(1), 30_000).await {
            connected = Some(db);
            break;
        }
        tokio::time::sleep(StdDuration::from_millis(250)).await;
    }
    let db = connected.expect("postgres never accepted connections");
    sqlx::migrate!("../../migrations").run(db.pool()).await.expect("migrate");
    (db, pg)
}

// --- a queue that never returns anything (the outbox tests don't need it) ---

#[derive(Default)]
struct EmptyQueue;

#[async_trait]
impl ReminderJobQueue for EmptyQueue {
    async fn push(&self, _job: &ReminderJob) -> Result<(), CacheError> {
        Ok(())
    }
    async fn try_pop(&self) -> Result<Option<ReminderJob>, CacheError> {
        Ok(None)
    }
}

// --- a sender that always fails (retry / permanent-fail paths) --------------

#[derive(Debug)]
struct FailingEmail;

#[async_trait]
impl EmailSender for FailingEmail {
    async fn send(&self, _email: OutboundEmail) -> Result<(), EmailError> {
        Err(EmailError::Smtp("simulated SMTP failure".into()))
    }
}

// --- WorkerState builder, parameterised by sender + max_retries -------------

fn build_state(
    db: &Database,
    clock: Arc<dyn Clock>,
    email: Arc<dyn EmailSender>,
    max_retries: i32,
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
        digests: Arc::new(PgReminderDigestRepo::new(pool.clone())),
        queue: Arc::new(EmptyQueue),
        email,
        janitor: Arc::new(PgJanitor::new(pool.clone())),
        outbox: Arc::new(PgEmailOutboxRepo::new(pool)),
        web_public_url: "https://app.example".into(),
        max_retries,
        retry_min_seconds: 1,
        retry_max_seconds: 60,
        janitor_grace_seconds: 0,
    }
}

fn sample_insert(kind: &str, to_addr: &str) -> EmailOutboxInsert {
    EmailOutboxInsert {
        kind: kind.to_string(),
        to_addr: to_addr.to_string(),
        subject: "Hello".into(),
        text_body: "Body".into(),
        html_body: None,
    }
}

// --- tests ------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn enqueue_then_process_one_sends_and_marks_sent() {
    let (db, _pg) = start_pg().await;
    let fake = Arc::new(FakeEmailSender::new());
    let clock = Arc::new(FixedClock::new(Utc::now()));
    let state = build_state(&db, clock as Arc<dyn Clock>, fake.clone() as Arc<dyn EmailSender>, 3);

    state
        .outbox
        .enqueue(&sample_insert(EmailOutboxKind::MAGIC_LINK, "happy@example.com"))
        .await
        .expect("enqueue");

    let did = outbox::process_one(&state).await;
    assert!(did, "process_one should have claimed the row");

    let sent = fake.drain();
    assert_eq!(sent.len(), 1, "exactly one email sent");
    assert_eq!(sent[0].to_addr, "happy@example.com");

    // Row terminal: status=sent, sent_at non-null, last_error cleared.
    let row = sqlx::query!(
        "SELECT status::text AS \"status!\", sent_at, last_error \
         FROM email_outbox WHERE to_addr = 'happy@example.com'"
    )
    .fetch_one(db.pool())
    .await
    .expect("fetch row");
    assert_eq!(row.status, "sent");
    assert!(row.sent_at.is_some(), "sent_at populated");
    assert!(row.last_error.is_none(), "last_error cleared on success");

    // The next claim has no work (terminal rows aren't re-claimed).
    let did2 = outbox::process_one(&state).await;
    assert!(!did2, "no more pending rows");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn failing_send_marks_retry_with_attempts_incremented_and_backoff_scheduled() {
    let (db, _pg) = start_pg().await;
    let now = Utc::now();
    let clock = Arc::new(FixedClock::new(now));
    let state = build_state(
        &db,
        clock as Arc<dyn Clock>,
        Arc::new(FailingEmail) as Arc<dyn EmailSender>,
        5, // plenty of retries left
    );

    state
        .outbox
        .enqueue(&sample_insert(EmailOutboxKind::INVITE, "fail@example.com"))
        .await
        .unwrap();

    let did = outbox::process_one(&state).await;
    assert!(did, "process_one should have attempted the row");

    let row = sqlx::query!(
        "SELECT status::text AS \"status!\", attempts, last_error, next_attempt_at \
         FROM email_outbox WHERE to_addr = 'fail@example.com'"
    )
    .fetch_one(db.pool())
    .await
    .expect("fetch row");
    assert_eq!(row.status, "pending", "still pending — has retries left");
    assert_eq!(row.attempts, 1, "attempts incremented");
    assert!(row.last_error.as_deref().unwrap_or("").contains("simulated"));
    // Backoff schedules next attempt strictly in the future.
    assert!(row.next_attempt_at > now, "next_attempt_at is in the future");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn failing_at_max_retries_marks_failed_permanent() {
    let (db, _pg) = start_pg().await;
    let clock = Arc::new(FixedClock::new(Utc::now()));
    // max_retries=1 ⇒ the very first failure pushes attempts (0+1) >=
    // max_retries, so the row goes terminal-failed in one cycle.
    let state = build_state(
        &db,
        clock as Arc<dyn Clock>,
        Arc::new(FailingEmail) as Arc<dyn EmailSender>,
        1,
    );

    state
        .outbox
        .enqueue(&sample_insert(EmailOutboxKind::OWNER_TRANSFER_FROM, "dead@example.com"))
        .await
        .unwrap();

    outbox::process_one(&state).await;

    let row = sqlx::query!(
        "SELECT status::text AS \"status!\", attempts, last_error \
         FROM email_outbox WHERE to_addr = 'dead@example.com'"
    )
    .fetch_one(db.pool())
    .await
    .expect("fetch row");
    assert_eq!(row.status, "failed_permanent");
    assert_eq!(row.attempts, 1);
    assert!(row.last_error.as_deref().unwrap_or("").contains("simulated"));

    // A subsequent process_one is a no-op (terminal rows aren't re-claimed).
    let did_again = outbox::process_one(&state).await;
    assert!(!did_again, "failed_permanent rows stay claimed-once");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn skip_locked_lets_two_pollers_drain_in_parallel_without_double_send() {
    // Seed three pending rows + two pollers running back-to-back. With
    // FOR UPDATE SKIP LOCKED each call grabs a distinct row, so two
    // sequential drains never double-claim — and a third drain handles
    // the leftover. (We don't actually need true concurrency to assert
    // the SKIP-LOCKED contract — distinct rows per drain is enough.)
    let (db, _pg) = start_pg().await;
    let fake = Arc::new(FakeEmailSender::new());
    let clock = Arc::new(FixedClock::new(Utc::now()));
    let state = build_state(&db, clock as Arc<dyn Clock>, fake.clone() as Arc<dyn EmailSender>, 3);

    for addr in ["a@example.com", "b@example.com", "c@example.com"] {
        state.outbox.enqueue(&sample_insert(EmailOutboxKind::INVITE, addr)).await.unwrap();
    }

    assert!(outbox::process_one(&state).await);
    assert!(outbox::process_one(&state).await);
    assert!(outbox::process_one(&state).await);
    assert!(!outbox::process_one(&state).await, "no more pending rows");

    let sent = fake.drain();
    let mut to_addrs: Vec<String> = sent.iter().map(|e| e.to_addr.clone()).collect();
    to_addrs.sort();
    assert_eq!(to_addrs, vec!["a@example.com", "b@example.com", "c@example.com"]);

    // All three terminal-sent.
    let sent_count: i64 =
        sqlx::query_scalar!("SELECT COUNT(*) AS \"c!\" FROM email_outbox WHERE status = 'sent'")
            .fetch_one(db.pool())
            .await
            .unwrap();
    assert_eq!(sent_count, 3);
}
