//! End-to-end repo smoke tests against a real Postgres via testcontainers.
//!
//! Each test spins a fresh Postgres container, runs the migration set, then
//! exercises one repo. The container value is bound in the test scope (NOT
//! `Box::leak`-ed) so its `Drop` impl runs when the test finishes — which
//! tells the testcontainers reaper to stop the container. Without that we
//! leak running postgres instances on the docker host across runs.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing, clippy::print_stderr)]

use std::time::Duration as StdDuration;

use chrono::{Duration, Utc};
use my_fam_tree_domain::{
    FamilyInviteRepo, FamilyMembershipRepo, FamilyRepo, Locale, MagicLinkPurpose, MagicLinkRepo,
    ParentKind, ParentLinkRepo, ParentLinkRepoError, PersonId, PersonRepo, RefreshTokenRepo, Role,
    UserRepo,
};
use my_fam_tree_persistence::{
    Database, PgFamilyInviteRepo, PgFamilyMembershipRepo, PgFamilyRepo, PgMagicLinkRepo,
    PgParentLinkRepo, PgPersonRepo, PgRefreshTokenRepo, PgUserRepo,
};
use sqlx::PgPool;
use testcontainers::ContainerAsync;
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::postgres::Postgres;

/// Bundles the pool + container so the test owns both. When `TestDb` drops,
/// the container's `Drop` impl asks the testcontainers reaper to stop and
/// remove the postgres container. The pool's drop is harmless.
struct TestDb {
    pool: PgPool,
    // Underscore prefix: never read, only held for lifetime side effect.
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

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn user_create_then_find() {
    let db = setup().await;
    let users = PgUserRepo::new(db.pool.clone());
    let user = users.create("a@b.c", Locale::En).await.expect("create");
    let found = users.find_by_email("a@b.c").await.expect("find").expect("some");
    assert_eq!(found.id, user.id);
    assert_eq!(found.locale, Locale::En);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn magic_link_consume_once() {
    let db = setup().await;
    let users = PgUserRepo::new(db.pool.clone());
    let mls = PgMagicLinkRepo::new(db.pool.clone());
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
    let db = setup().await;
    let users = PgUserRepo::new(db.pool.clone());
    let fams = PgFamilyRepo::new(db.pool.clone());
    let mems = PgFamilyMembershipRepo::new(db.pool.clone());
    let u1 = users.create("a@b.c", Locale::En).await.unwrap();
    let u2 = users.create("c@d.e", Locale::En).await.unwrap();
    let fam = fams.create("Müller", u1.id).await.unwrap();
    mems.insert(fam.id, u1.id, Role::Owner).await.unwrap();
    let res = mems.insert(fam.id, u2.id, Role::Owner).await;
    assert!(matches!(res, Err(my_fam_tree_domain::MembershipRepoError::OwnerExists)));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn refresh_token_rotation_preserves_absolute_expiry() {
    let db = setup().await;
    let users = PgUserRepo::new(db.pool.clone());
    let rts = PgRefreshTokenRepo::new(db.pool.clone());
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
async fn parent_links_db_trigger_rejects_cycle() {
    // Sanity-check that the BEFORE-INSERT trigger added in
    // migration 0003 fires even when the in-memory check is bypassed.
    // We pre-load a (child=A, parent=B) edge, then try to insert
    // (child=B, parent=A) — the in-memory check WILL catch this too,
    // but more importantly the trigger would catch it even if a
    // concurrent writer slipped past the SERIALIZABLE snapshot.
    use my_fam_tree_domain::PersonDraft;

    let db = setup().await;
    let users = PgUserRepo::new(db.pool.clone());
    let fams = PgFamilyRepo::new(db.pool.clone());
    let persons = PgPersonRepo::new(db.pool.clone());
    let pls = PgParentLinkRepo::new(db.pool.clone());

    let owner = users.create("cyc@x.y", Locale::En).await.unwrap();
    let fam = fams.create("Cycle Test", owner.id).await.unwrap();

    let a = persons
        .create(fam.id, PersonDraft { given_name: "A".into(), ..PersonDraft::default() })
        .await
        .unwrap();
    let b = persons
        .create(fam.id, PersonDraft { given_name: "B".into(), ..PersonDraft::default() })
        .await
        .unwrap();

    // Edge 1: A is child of B.
    pls.insert(fam.id, a.id, b.id, ParentKind::Biological, "").await.expect("first edge ok");

    // Edge 2 would close the cycle. The repo's in-memory check returns
    // Cycle first; that's correct behaviour. The DB trigger is the
    // race-safe backstop and we verify its existence by INSERTing the
    // edge directly via raw SQL (bypassing the repo) — that path
    // exercises only the trigger.
    let raw = sqlx::query(
        "INSERT INTO parent_links (child_id, parent_id, kind, note) \
         VALUES ($1, $2, 'biological'::parent_link_kind, '')",
    )
    .bind(PersonId::into_uuid(b.id))
    .bind(PersonId::into_uuid(a.id))
    .execute(&db.pool)
    .await;
    let err = raw.expect_err("DB trigger must reject the cycle");
    let db_err = err.as_database_error().expect("expected database error");
    assert_eq!(db_err.code().as_deref(), Some("23514"));
    assert!(db_err.message().contains("parent_links cycle"));

    // Now go through the repo: in-memory check fires, returning Cycle.
    let via_repo = pls.insert(fam.id, b.id, a.id, ParentKind::Biological, "").await;
    assert!(matches!(via_repo, Err(ParentLinkRepoError::Cycle)));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn invite_accept_is_idempotent() {
    let db = setup().await;
    let users = PgUserRepo::new(db.pool.clone());
    let fams = PgFamilyRepo::new(db.pool.clone());
    let mems = PgFamilyMembershipRepo::new(db.pool.clone());
    let invs = PgFamilyInviteRepo::new(db.pool.clone());
    let owner = users.create("o@x.y", Locale::En).await.unwrap();
    let fam = fams.create("Schmidt", owner.id).await.unwrap();
    mems.insert(fam.id, owner.id, Role::Owner).await.unwrap();
    let hash = [7_u8; 32];
    invs.create(
        fam.id,
        "new@x.y",
        Role::User,
        owner.id,
        None,
        &hash,
        Utc::now() + Duration::days(14),
    )
    .await
    .unwrap();
    let inv = invs.accept(&hash, Utc::now()).await.expect("accept");
    assert_eq!(inv.email, "new@x.y");
    let second = invs.accept(&hash, Utc::now()).await;
    assert!(matches!(second, Err(my_fam_tree_domain::InviteRepoError::NotFoundOrAccepted)));
}
