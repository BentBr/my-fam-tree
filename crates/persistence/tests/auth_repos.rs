//! End-to-end repo smoke tests against a real Postgres via testcontainers.
//!
//! Each test spins a fresh Postgres container, runs the migration set, then
//! exercises one repo. The container is `Box::leak`-ed for the duration of the
//! test so its docker handle outlives the pool. Containers are reaped when the
//! test process exits.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing, clippy::print_stderr)]

use std::time::Duration as StdDuration;

use chrono::{Duration, Utc};
use my_family_domain::{
    FamilyInviteRepo, FamilyMembershipRepo, FamilyRepo, Locale, MagicLinkPurpose, MagicLinkRepo,
    RefreshTokenRepo, Role, UserRepo,
};
use my_family_persistence::{
    Database, PgFamilyInviteRepo, PgFamilyMembershipRepo, PgFamilyRepo, PgMagicLinkRepo,
    PgRefreshTokenRepo, PgUserRepo,
};
use sqlx::PgPool;
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::postgres::Postgres;

async fn setup() -> PgPool {
    let container = Postgres::default()
        .with_db_name("test")
        .with_user("test")
        .with_password("test")
        .start()
        .await
        .expect("start pg");
    let port = container.get_host_port_ipv4(5432_u16).await.expect("port");
    let url = format!("postgres://test:test@127.0.0.1:{port}/test");

    // Wait for ready.
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

    // Box-leak container so it lives for the test.
    Box::leak(Box::new(container));
    db.pool().clone()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn user_create_then_find() {
    let pool = setup().await;
    let users = PgUserRepo::new(pool);
    let user = users.create("a@b.c", Locale::En).await.expect("create");
    let found = users.find_by_email("a@b.c").await.expect("find").expect("some");
    assert_eq!(found.id, user.id);
    assert_eq!(found.locale, Locale::En);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn magic_link_consume_once() {
    let pool = setup().await;
    let users = PgUserRepo::new(pool.clone());
    let mls = PgMagicLinkRepo::new(pool);
    let u = users.create("a@b.c", Locale::En).await.unwrap();
    let hash = [0_u8; 32];
    mls.create(
        Some(u.id),
        &u.email,
        &hash,
        MagicLinkPurpose::Login,
        Utc::now() + Duration::minutes(5),
    )
    .await
    .unwrap();
    let rec = mls.consume(&hash).await.expect("first consume");
    assert_eq!(rec.user_id, Some(u.id));
    let second = mls.consume(&hash).await;
    assert!(second.is_err());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn family_owner_uniqueness_enforced() {
    let pool = setup().await;
    let users = PgUserRepo::new(pool.clone());
    let fams = PgFamilyRepo::new(pool.clone());
    let mems = PgFamilyMembershipRepo::new(pool);
    let u1 = users.create("a@b.c", Locale::En).await.unwrap();
    let u2 = users.create("c@d.e", Locale::En).await.unwrap();
    let fam = fams.create("Müller", u1.id).await.unwrap();
    mems.insert(fam.id, u1.id, Role::Owner).await.unwrap();
    let res = mems.insert(fam.id, u2.id, Role::Owner).await;
    assert!(matches!(res, Err(my_family_domain::MembershipRepoError::OwnerExists)));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn refresh_token_rotation_preserves_absolute_expiry() {
    let pool = setup().await;
    let users = PgUserRepo::new(pool.clone());
    let rts = PgRefreshTokenRepo::new(pool);
    let u = users.create("x@y.z", Locale::De).await.unwrap();
    let abs_exp = Utc::now() + Duration::days(90);
    let h1 = [1_u8; 32];
    let h2 = [2_u8; 32];
    rts.create(u.id, &h1, None, None, None, Utc::now() + Duration::days(30), abs_exp)
        .await
        .unwrap();
    rts.rotate(&h1, &h2, Utc::now() + Duration::days(30), None, None, None).await.unwrap();
    let new_rec = rts.find_active_by_hash(&h2).await.unwrap().expect("found");
    assert_eq!(new_rec.absolute_expires_at.timestamp(), abs_exp.timestamp());
    assert!(rts.find_active_by_hash(&h1).await.unwrap().is_none());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn invite_accept_is_idempotent() {
    let pool = setup().await;
    let users = PgUserRepo::new(pool.clone());
    let fams = PgFamilyRepo::new(pool.clone());
    let mems = PgFamilyMembershipRepo::new(pool.clone());
    let invs = PgFamilyInviteRepo::new(pool);
    let owner = users.create("o@x.y", Locale::En).await.unwrap();
    let fam = fams.create("Schmidt", owner.id).await.unwrap();
    mems.insert(fam.id, owner.id, Role::Owner).await.unwrap();
    let hash = [7_u8; 32];
    invs.create(fam.id, "new@x.y", Role::User, owner.id, &hash, Utc::now() + Duration::days(14))
        .await
        .unwrap();
    let inv = invs.accept(&hash, Utc::now()).await.expect("accept");
    assert_eq!(inv.email, "new@x.y");
    let second = invs.accept(&hash, Utc::now()).await;
    assert!(matches!(second, Err(my_family_domain::InviteRepoError::NotFoundOrAccepted)));
}
