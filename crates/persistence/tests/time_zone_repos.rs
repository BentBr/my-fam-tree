//! Postgres-resolved timezone logic: `ReminderPreferencesRepo::due_at_local_hour`
//! (which users are in their local digest hour, and their local date) and
//! `PgTimeZoneRepo`. Runs against a real Postgres (testcontainers) because
//! the whole point is that the tz database + DST rules live there.

#![expect(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test code: setup failures abort the test; setup failures abort the test with context"
)]

use std::time::Duration as StdDuration;

use my_fam_tree_domain::dates::{DEFAULT_TIMEZONE, DIGEST_LOCAL_HOUR};
use my_fam_tree_domain::{
    Locale, ReminderPreferences, ReminderPreferencesRepo, TimeZoneRepo, TimeZoneRepoError, UserId,
    UserRepo,
};
use my_fam_tree_persistence::{Database, PgReminderPrefsRepo, PgTimeZoneRepo, PgUserRepo};
use sqlx::PgPool;
use testcontainers::ContainerAsync;
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::postgres::Postgres;
use time::macros::{date, datetime, time};
use time::{Date, OffsetDateTime};

struct TestDb {
    pool: PgPool,
    _container: ContainerAsync<Postgres>,
}

async fn setup() -> TestDb {
    let container = Postgres::default()
        .with_db_name("test")
        .with_user("test")
        .with_password("test")
        .start()
        .await
        .expect("start pg");
    let port = container.get_host_port_ipv4(5432_u16).await.expect("port");
    let url = format!("postgres://test:test@127.0.0.1:{port}/test");

    let mut connected = None;
    for _ in 0..40 {
        if let Ok(db) = Database::connect(&url, 2, StdDuration::from_secs(1), 30_000).await {
            connected = Some(db);
            break;
        }
        tokio::time::sleep(StdDuration::from_millis(250)).await;
    }
    let db = connected.expect("postgres never accepted connections");
    sqlx::migrate!("../../migrations").run(db.pool()).await.expect("migrate");
    TestDb { pool: db.pool().clone(), _container: container }
}

const fn prefs(emails_enabled: bool) -> ReminderPreferences {
    ReminderPreferences {
        emails_enabled,
        remind_birthdays: true,
        remind_anniversaries: true,
        favourites_only: false,
        lead_days: 7,
    }
}

/// Create a user with reminders on/off and the given stored timezone.
async fn user(pool: &PgPool, email: &str, tz: &str, enabled: bool) -> UserId {
    let u = PgUserRepo::new(pool.clone()).create(email, Locale::En).await.unwrap();
    sqlx::query!("UPDATE users SET timezone = $1 WHERE id = $2", tz, u.id.into_uuid())
        .execute(pool)
        .await
        .unwrap();
    PgReminderPrefsRepo::new(pool.clone()).upsert(u.id, prefs(enabled)).await.unwrap();
    u.id
}

/// `(user, local_date)` pairs due at `now`, sorted by user id like the repo.
async fn due(pool: &PgPool, now: OffsetDateTime) -> Vec<(UserId, Date)> {
    PgReminderPrefsRepo::new(pool.clone())
        .due_at_local_hour(now, DIGEST_LOCAL_HOUR, DEFAULT_TIMEZONE)
        .await
        .unwrap()
        .into_iter()
        .map(|d| (d.user_id, d.local_date))
        .collect()
}

fn sorted(mut v: Vec<(UserId, Date)>) -> Vec<(UserId, Date)> {
    v.sort_by_key(|(id, _)| id.into_uuid());
    v
}

// --- due_at_local_hour ----------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn due_users_follow_their_own_timezone() {
    let db = setup().await;
    let p = &db.pool;
    let berlin = user(p, "berlin@x.io", "Europe/Berlin", true).await;
    let ny = user(p, "ny@x.io", "America/New_York", true).await;
    let kolkata = user(p, "kolkata@x.io", "Asia/Kolkata", true).await; // UTC+05:30
    let _disabled = user(p, "off@x.io", "Europe/Berlin", false).await;

    // 04:00Z = 06:00 CEST (Berlin) = 00:00 EDT (NY) = 09:30 IST (Kolkata).
    let d = date!(2026 - 06 - 08);
    assert_eq!(due(p, datetime!(2026-06-08 04:00 UTC)).await, vec![(berlin, d)]);
    // 10:00Z = 06:00 EDT.
    assert_eq!(due(p, datetime!(2026-06-08 10:00 UTC)).await, vec![(ny, d)]);
    // 00:30Z = 06:00 IST — half-hour offsets work.
    assert_eq!(due(p, datetime!(2026-06-08 00:30 UTC)).await, vec![(kolkata, d)]);
    // 12:00Z = 14:00 Berlin / 08:00 NY / 17:30 Kolkata — nobody.
    assert_eq!(due(p, datetime!(2026-06-08 12:00 UTC)).await, Vec::<(UserId, Date)>::new());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn emails_disabled_users_are_never_due() {
    let db = setup().await;
    user(&db.pool, "off@x.io", "Europe/Berlin", false).await;
    assert_eq!(due(&db.pool, datetime!(2026-06-08 04:00 UTC)).await, Vec::<(UserId, Date)>::new());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn unknown_timezone_falls_back_to_default() {
    let db = setup().await;
    let p = &db.pool;
    let bogus = user(p, "bogus@x.io", "Not/AZone", true).await;
    let empty = user(p, "empty@x.io", "", true).await;
    let berlin = user(p, "berlin@x.io", "Europe/Berlin", true).await;

    let d = date!(2026 - 06 - 08);
    let got = due(p, datetime!(2026-06-08 04:00 UTC)).await;
    assert_eq!(got, sorted(vec![(bogus, d), (empty, d), (berlin, d)]), "all resolve to Berlin");
    assert!(due(p, datetime!(2026-06-08 10:00 UTC)).await.is_empty(), "not NY time");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn hour_window_is_inclusive_start_exclusive_end() {
    let db = setup().await;
    let p = &db.pool;
    let berlin = user(p, "berlin@x.io", "Europe/Berlin", true).await;
    let d = date!(2026 - 06 - 08);

    assert!(due(p, datetime!(2026-06-08 03:59:59 UTC)).await.is_empty(), "05:59:59 local");
    assert_eq!(due(p, datetime!(2026-06-08 04:00:00 UTC)).await, vec![(berlin, d)]);
    assert_eq!(due(p, datetime!(2026-06-08 04:59:59.999 UTC)).await, vec![(berlin, d)]);
    assert!(due(p, datetime!(2026-06-08 05:00:00 UTC)).await.is_empty(), "07:00 local");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn dst_transitions_shift_the_utc_firing_time() {
    let db = setup().await;
    let p = &db.pool;
    let berlin = user(p, "berlin@x.io", "Europe/Berlin", true).await;

    // Day before spring-forward: CET (UTC+1) → 06:00 local = 05:00Z.
    assert_eq!(
        due(p, datetime!(2026-03-28 05:00 UTC)).await,
        vec![(berlin, date!(2026 - 03 - 28))]
    );
    assert_eq!(due(p, datetime!(2026-03-28 04:00 UTC)).await, Vec::<(UserId, Date)>::new());
    // Spring-forward day (02:00→03:00 on 2026-03-29): CEST → 06:00 = 04:00Z.
    assert_eq!(
        due(p, datetime!(2026-03-29 04:00 UTC)).await,
        vec![(berlin, date!(2026 - 03 - 29))]
    );
    assert!(due(p, datetime!(2026-03-29 05:00 UTC)).await.is_empty(), "07:00 CEST");
    // Fall-back day (03:00→02:00 on 2026-10-25): CET again → 05:00Z.
    assert_eq!(
        due(p, datetime!(2026-10-25 05:00 UTC)).await,
        vec![(berlin, date!(2026 - 10 - 25))]
    );
    assert!(due(p, datetime!(2026-10-25 04:00 UTC)).await.is_empty(), "05:00 CET");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn local_date_can_differ_from_the_utc_date() {
    let db = setup().await;
    let p = &db.pool;
    // Auckland in June is NZST (UTC+12): 06:00 on the 8th is 18:00Z on the 7th.
    let akl = user(p, "akl@x.io", "Pacific/Auckland", true).await;
    assert_eq!(
        due(p, datetime!(2026-06-07 18:00 UTC)).await,
        vec![(akl, date!(2026 - 06 - 08))],
        "local date is the 8th although UTC is still the 7th"
    );
}

// --- PgTimeZoneRepo -------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn local_date_resolves_in_the_given_zone() {
    let db = setup().await;
    let zones = PgTimeZoneRepo::new(db.pool.clone());
    let at = datetime!(2026-06-07 23:30 UTC);
    assert_eq!(zones.local_date(at, "Europe/Berlin").await.unwrap(), date!(2026 - 06 - 08));
    assert_eq!(zones.local_date(at, "UTC").await.unwrap(), date!(2026 - 06 - 07));
    assert_eq!(zones.local_date(at, "America/New_York").await.unwrap(), date!(2026 - 06 - 07));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn local_to_utc_is_dst_aware() {
    let db = setup().await;
    let zones = PgTimeZoneRepo::new(db.pool.clone());
    let six = time!(06:00);
    let summer = zones.local_to_utc(date!(2026 - 06 - 08), six, "Europe/Berlin").await.unwrap();
    assert_eq!(summer, datetime!(2026-06-08 04:00 UTC), "CEST = UTC+2");
    let winter = zones.local_to_utc(date!(2026 - 01 - 08), six, "Europe/Berlin").await.unwrap();
    assert_eq!(winter, datetime!(2026-01-08 05:00 UTC), "CET = UTC+1");
    let ny = zones.local_to_utc(date!(2026 - 06 - 08), six, "America/New_York").await.unwrap();
    assert_eq!(ny, datetime!(2026-06-08 10:00 UTC), "EDT = UTC-4");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn local_to_utc_in_spring_forward_gap_shifts_forward() {
    let db = setup().await;
    let zones = PgTimeZoneRepo::new(db.pool.clone());
    // 02:30 doesn't exist in Berlin on 2026-03-29; Postgres reads it with
    // the pre-transition offset (UTC+1), i.e. 01:30Z = 03:30 CEST.
    let got =
        zones.local_to_utc(date!(2026 - 03 - 29), time!(02:30), "Europe/Berlin").await.unwrap();
    assert_eq!(got, datetime!(2026-03-29 01:30 UTC));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn unknown_zone_is_a_typed_error() {
    let db = setup().await;
    let zones = PgTimeZoneRepo::new(db.pool.clone());
    let err = zones.local_date(datetime!(2026-06-08 04:00 UTC), "Not/AZone").await.unwrap_err();
    assert!(matches!(err, TimeZoneRepoError::UnknownZone(ref z) if z == "Not/AZone"), "{err:?}");
    let err2 = zones.local_to_utc(date!(2026 - 06 - 08), time!(06:00), "Nope").await.unwrap_err();
    assert!(matches!(err2, TimeZoneRepoError::UnknownZone(ref z) if z == "Nope"), "{err2:?}");
}
