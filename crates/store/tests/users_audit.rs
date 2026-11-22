//! Users and the audit log, each test on a fresh in-memory database.

use booker_core::{Floor, FloorId, Role, User, UserId, ValidationError};
use booker_store::{FloorRepo, SqliteStore, StoreError};

async fn store() -> SqliteStore {
    SqliteStore::open_in_memory().await.unwrap()
}

fn user(name: &str, role: Role) -> User {
    User {
        id: UserId::new(),
        name: name.into(),
        role,
        active: true,
    }
}

fn floor(name: &str) -> Floor {
    Floor {
        id: FloorId::new(),
        name: name.into(),
        level: 0,
        grid_cols: 10,
        grid_rows: 10,
    }
}

#[tokio::test]
async fn users_are_saved_with_a_pin_hash_and_listed() {
    let s = store().await;
    let andy = user("Andy", Role::Admin);
    s.save_user(&andy, Some("$argon2id$fake")).await.unwrap();
    assert_eq!(s.list_users().await.unwrap(), vec![andy.clone()]);
    assert_eq!(
        s.pin_hash(andy.id).await.unwrap().as_deref(),
        Some("$argon2id$fake")
    );

    // Updating without a new PIN keeps the old one.
    let mut renamed = andy.clone();
    renamed.name = "Andrew".into();
    s.save_user(&renamed, None).await.unwrap();
    assert_eq!(
        s.pin_hash(andy.id).await.unwrap().as_deref(),
        Some("$argon2id$fake")
    );
}

#[tokio::test]
async fn a_new_user_needs_a_pin_and_the_last_admin_stays() {
    let s = store().await;
    let andy = user("Andy", Role::Admin);
    assert!(matches!(
        s.save_user(&andy, None).await,
        Err(StoreError::NotFound { .. })
    ));
    s.save_user(&andy, Some("h")).await.unwrap();

    let mut demoted = andy.clone();
    demoted.role = Role::Staff;
    let err = s.save_user(&demoted, None).await.unwrap_err();
    assert!(matches!(
        err,
        StoreError::Validation(ValidationError::LastAdmin)
    ));
}

#[tokio::test]
async fn inactive_users_have_no_usable_pin() {
    let s = store().await;
    let andy = user("Andy", Role::Admin);
    let mut sam = user("Sam", Role::Staff);
    s.save_user(&andy, Some("h1")).await.unwrap();
    s.save_user(&sam, Some("h2")).await.unwrap();
    sam.active = false;
    s.save_user(&sam, None).await.unwrap();
    assert_eq!(s.pin_hash(sam.id).await.unwrap(), None);
}

#[tokio::test]
async fn changes_are_logged_against_the_acting_user() {
    let s = store().await;
    let andy = user("Andy", Role::Admin);
    s.save_user(&andy, Some("h")).await.unwrap();

    let as_andy = s.as_user(&andy);
    let mut f = floor("Ground");
    as_andy.save_floor(&f).await.unwrap();
    f.name = "Ground floor".into();
    as_andy.save_floor(&f).await.unwrap();
    as_andy.delete_floor(f.id).await.unwrap();

    let log = s
        .list_audit(Some(andy.id), Some("floor"), 50)
        .await
        .unwrap();
    let summaries: Vec<_> = log.iter().map(|e| e.summary.as_str()).collect();
    assert_eq!(
        summaries,
        vec![
            "Deleted floor Ground floor",
            "Changed floor Ground floor",
            "Added floor Ground"
        ]
    );
    let changed = &log[1];
    assert_eq!(changed.user_name, "Andy");
    assert!(changed.before.as_deref().unwrap().contains("\"Ground\""));
    assert!(
        changed
            .after
            .as_deref()
            .unwrap()
            .contains("\"Ground floor\"")
    );
}

#[tokio::test]
async fn changes_without_a_user_are_logged_as_system() {
    let s = store().await;
    s.save_floor(&floor("Ground")).await.unwrap();
    let log = s.list_audit(None, None, 10).await.unwrap();
    assert_eq!(log.len(), 1);
    assert_eq!(log[0].user_name, "System");
    assert_eq!(log[0].user_id, None);
}

#[tokio::test]
async fn a_rejected_change_leaves_no_log_entry() {
    let s = store().await;
    let mut bad = floor("Ground");
    bad.grid_cols = 0;
    assert!(s.save_floor(&bad).await.is_err());
    assert!(s.list_audit(None, None, 10).await.unwrap().is_empty());
}

#[tokio::test]
async fn loading_the_demo_is_in_the_log() {
    let s = store().await;
    s.seed_demo().await.unwrap();
    let setup = s.list_audit(None, Some("setup"), 10).await.unwrap();
    assert!(
        setup
            .iter()
            .any(|e| e.summary.starts_with("Loaded the demo hotel"))
    );
    assert!(
        s.list_audit(None, Some("nothing"), 10)
            .await
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn email_settings_templates_and_log_round_trip() {
    use booker_core::{EmailKind, EmailLogEntry, EmailSettings};
    use booker_store::EmailRepo;
    let store = SqliteStore::open_in_memory().await.unwrap();
    assert_eq!(store.get_email_settings().await.unwrap().port, 587);
    let gmail = EmailSettings {
        host: "smtp.gmail.com".into(),
        port: 587,
        username: "grange@gmail.com".into(),
        from_address: "grange@gmail.com".into(),
        from_name: "The Grange".into(),
    };
    store.save_email_settings(&gmail).await.unwrap();
    assert_eq!(store.get_email_settings().await.unwrap(), gmail);

    // Templates start as the defaults and can be reworded one at a time.
    let mut templates = store.list_email_templates().await.unwrap();
    assert_eq!(templates.len(), 3);
    templates[0].subject = "See you soon, {first_name}".into();
    store.save_email_template(&templates[0]).await.unwrap();
    let again = store.list_email_templates().await.unwrap();
    assert_eq!(again[0].subject, "See you soon, {first_name}");
    assert_eq!(again[1].kind, EmailKind::Amendment);

    let entry = EmailLogEntry {
        at: chrono::NaiveDateTime::parse_from_str("2026-06-01 10:00:00", "%Y-%m-%d %H:%M:%S")
            .unwrap(),
        to: "jo@example.com".into(),
        subject: "Test".into(),
        kind: None,
        booking_id: None,
        sent: false,
        error: "No password".into(),
        sent_by: "Andy".into(),
    };
    store.log_email(&entry).await.unwrap();
    assert_eq!(store.list_email_log(None).await.unwrap(), vec![entry]);
}
