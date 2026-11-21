//! Integration tests: each test gets its own fresh in-memory database.

use booker_core::{
    Floor, FloorId, GridRect, PropertySettings, RoomType, RoomTypeId, Space, SpaceId, SpaceKind,
    SuiteCapacities, ValidationError,
};
use booker_store::{FloorRepo, RoomTypeRepo, SettingsRepo, SpaceRepo, SqliteStore, StoreError};

async fn store() -> SqliteStore {
    SqliteStore::open_in_memory()
        .await
        .expect("open in-memory db")
}

fn floor(name: &str, level: i32) -> Floor {
    Floor {
        id: FloorId::new(),
        name: name.into(),
        level,
        grid_cols: 20,
        grid_rows: 10,
    }
}

fn double() -> RoomType {
    RoomType {
        id: RoomTypeId::new(),
        name: "Double".into(),
        code: "dbl".into(),
        default_occupancy: 2,
        max_occupancy: 2,
        description: String::new(),
        base_rate: booker_core::Money::from_major(90),
        extra_adult_rate: booker_core::Money::ZERO,
        child_rate: booker_core::Money::ZERO,
    }
}

fn room(floor: &Floor, rt: &RoomType, name: &str, rect: GridRect) -> Space {
    Space {
        id: SpaceId::new(),
        floor_id: floor.id,
        kind: SpaceKind::Room,
        name: name.into(),
        room_type_id: Some(rt.id),
        suite_capacities: SuiteCapacities::default(),
        rect,
        out_of_service: false,
        notes: String::new(),
    }
}

/// Builds a store containing one floor and one room type.
async fn with_floor_and_type() -> (SqliteStore, Floor, RoomType) {
    let s = store().await;
    let f = floor("Ground", 0);
    let rt = double();
    s.save_floor(&f).await.unwrap();
    s.save_room_type(&rt).await.unwrap();
    (s, f, rt)
}

#[tokio::test]
async fn settings_default_then_round_trip() {
    let s = store().await;
    assert_eq!(s.get_settings().await.unwrap(), PropertySettings::default());

    let mut settings = PropertySettings {
        hotel_name: "The Grange".into(),
        ..Default::default()
    };
    settings.currency = "EUR".into();
    s.save_settings(&settings).await.unwrap();
    s.save_settings(&settings).await.unwrap(); // second save updates the same row
    assert_eq!(s.get_settings().await.unwrap(), settings);
}

#[tokio::test]
async fn invalid_settings_are_rejected() {
    let s = store().await;
    let bad = PropertySettings {
        currency: "pounds".into(),
        ..Default::default()
    };
    let err = s.save_settings(&bad).await.unwrap_err();
    assert!(matches!(
        err,
        StoreError::Validation(ValidationError::InvalidCurrency)
    ));
}

#[tokio::test]
async fn floors_are_listed_by_level_and_can_be_updated() {
    let s = store().await;
    let first = floor("First", 1);
    let mut ground = floor("Ground", 0);
    s.save_floor(&first).await.unwrap();
    s.save_floor(&ground).await.unwrap();

    ground.grid_cols = 30;
    s.save_floor(&ground).await.unwrap();

    let floors = s.list_floors().await.unwrap();
    assert_eq!(floors, vec![ground, first]);
}

#[tokio::test]
async fn room_type_codes_are_stored_uppercase() {
    let (s, _, rt) = with_floor_and_type().await;
    let saved = s.list_room_types().await.unwrap();
    assert_eq!(saved.len(), 1);
    assert_eq!(saved[0].code, "DBL");
    assert_eq!(saved[0].id, rt.id);
}

#[tokio::test]
async fn spaces_round_trip_including_suite_capacities() {
    let (s, f, rt) = with_floor_and_type().await;
    let r = room(&f, &rt, "101", GridRect::new(0, 0, 3, 4));
    let suite = Space {
        kind: SpaceKind::Suite,
        name: "Garden Suite".into(),
        room_type_id: None,
        suite_capacities: SuiteCapacities {
            theatre: Some(120),
            ..Default::default()
        },
        rect: GridRect::new(5, 5, 8, 4),
        ..room(&f, &rt, "", GridRect::new(0, 0, 1, 1))
    };
    s.save_space(&r).await.unwrap();
    s.save_space(&suite).await.unwrap();

    let on_floor = s.list_spaces(Some(f.id)).await.unwrap();
    assert_eq!(on_floor, vec![r, suite]); // ordered by kind (room < suite), then name
    assert!(
        s.list_spaces(Some(FloorId::new()))
            .await
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn overlapping_space_is_rejected_and_nothing_is_saved() {
    let (s, f, rt) = with_floor_and_type().await;
    s.save_space(&room(&f, &rt, "101", GridRect::new(0, 0, 3, 4)))
        .await
        .unwrap();

    let err = s
        .save_space(&room(&f, &rt, "102", GridRect::new(2, 0, 3, 4)))
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        StoreError::Validation(ValidationError::Overlaps { .. })
    ));
    assert_eq!(s.list_spaces(None).await.unwrap().len(), 1);
}

#[tokio::test]
async fn space_on_unknown_floor_or_type_is_not_found() {
    let (s, f, rt) = with_floor_and_type().await;
    let mut r = room(&f, &rt, "101", GridRect::new(0, 0, 3, 4));
    r.floor_id = FloorId::new();
    assert!(matches!(
        s.save_space(&r).await,
        Err(StoreError::NotFound { what: "Floor" })
    ));

    r.floor_id = f.id;
    r.room_type_id = Some(RoomTypeId::new());
    assert!(matches!(
        s.save_space(&r).await,
        Err(StoreError::NotFound { what: "Room type" })
    ));
}

#[tokio::test]
async fn floor_with_rooms_cannot_be_deleted_until_emptied() {
    let (s, f, rt) = with_floor_and_type().await;
    let r = room(&f, &rt, "101", GridRect::new(0, 0, 3, 4));
    s.save_space(&r).await.unwrap();

    let err = s.delete_floor(f.id).await.unwrap_err();
    assert!(matches!(
        err,
        StoreError::Validation(ValidationError::FloorNotEmpty { count: 1, .. })
    ));

    s.delete_space(r.id).await.unwrap();
    s.delete_floor(f.id).await.unwrap();
    assert!(s.list_floors().await.unwrap().is_empty());
    assert!(matches!(
        s.delete_floor(f.id).await,
        Err(StoreError::NotFound { .. })
    ));
}

#[tokio::test]
async fn room_type_in_use_cannot_be_deleted() {
    let (s, f, rt) = with_floor_and_type().await;
    s.save_space(&room(&f, &rt, "101", GridRect::new(0, 0, 3, 4)))
        .await
        .unwrap();
    let err = s.delete_room_type(rt.id).await.unwrap_err();
    assert!(matches!(
        err,
        StoreError::Validation(ValidationError::RoomTypeInUse { .. })
    ));
}

#[tokio::test]
async fn deleted_space_name_can_be_reused() {
    let (s, f, rt) = with_floor_and_type().await;
    let r = room(&f, &rt, "101", GridRect::new(0, 0, 3, 4));
    s.save_space(&r).await.unwrap();
    s.delete_space(r.id).await.unwrap();
    s.save_space(&room(&f, &rt, "101", GridRect::new(0, 0, 3, 4)))
        .await
        .unwrap();
}

#[tokio::test]
async fn seed_demo_builds_a_hotel_once() {
    let s = store().await;
    assert!(s.seed_demo().await.unwrap());
    assert!(!s.seed_demo().await.unwrap(), "second call is a no-op");

    assert_eq!(s.list_floors().await.unwrap().len(), 3);
    assert_eq!(s.list_room_types().await.unwrap().len(), 4);
    let spaces = s.list_spaces(None).await.unwrap();
    let rooms = spaces.iter().filter(|x| x.kind == SpaceKind::Room).count();
    let suites = spaces.iter().filter(|x| x.kind == SpaceKind::Suite).count();
    assert_eq!((rooms, suites), (40, 2));
}

#[tokio::test]
async fn file_database_persists_across_reopen() {
    let dir = std::env::temp_dir().join(format!("booker-test-{}", uuid::Uuid::now_v7()));
    let path = dir.join("booker.db");
    {
        let s = SqliteStore::open(&path).await.unwrap();
        s.seed_demo().await.unwrap();
    }
    let s = SqliteStore::open(&path).await.unwrap();
    assert_eq!(s.list_spaces(None).await.unwrap().len(), 42);
    drop(s);
    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn backups_are_written_and_rotated() {
    // A file database: VACUUM INTO from an in-memory one writes to memory too.
    let root = std::env::temp_dir().join(format!("booker-backup-test-{}", uuid::Uuid::now_v7()));
    let dir = root.join("backups");
    let s = SqliteStore::open(&root.join("booker.db")).await.unwrap();
    s.seed_demo().await.unwrap();
    for _ in 0..4 {
        s.backup_with_rotation(&dir, 2).await.unwrap();
    }
    let mut files: Vec<_> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    files.sort();
    assert_eq!(files.len(), 2, "only the newest two are kept");

    let restored = SqliteStore::open(&files[1]).await.unwrap();
    assert_eq!(restored.list_spaces(None).await.unwrap().len(), 42);
    drop(restored);
    drop(s);
    let _ = std::fs::remove_dir_all(&root);
}

#[tokio::test]
async fn delete_the_live_data_restore_a_backup_and_it_is_all_back() {
    use booker_store::{BackupRepo, restore_file};
    let root = std::env::temp_dir().join(format!("booker-restore-test-{}", uuid::Uuid::now_v7()));
    let db = root.join("booker.db");
    let dir = root.join("backups");
    let s = SqliteStore::open(&db).await.unwrap();
    s.seed_demo().await.unwrap();
    let settings = s.get_backup_settings().await.unwrap();
    assert_eq!((settings.keep_daily, settings.keep_monthly), (14, 12));
    let backup = s.backup_now(&dir, &settings).await.unwrap();
    assert!(backup.size_bytes > 0);
    assert_eq!(booker_store::list_backups(&dir).unwrap().len(), 1);

    // Lose everything after the backup: a new floor, then the whole file.
    let floor = Floor {
        id: FloorId::new(),
        name: "Annexe".into(),
        level: 9,
        grid_cols: 10,
        grid_rows: 10,
    };
    s.save_floor(&floor).await.unwrap();
    s.close().await;
    std::fs::remove_file(&db).unwrap();

    restore_file(&dir.join(&backup.name), &db).unwrap();
    let back = SqliteStore::open(&db).await.unwrap();
    assert_eq!(back.list_spaces(None).await.unwrap().len(), 42);
    assert!(
        !back
            .list_floors()
            .await
            .unwrap()
            .iter()
            .any(|f| f.name == "Annexe")
    );
    // A file that isn't a backup is refused.
    std::fs::write(root.join("notes.txt"), "hello").unwrap();
    assert!(restore_file(&root.join("notes.txt"), &db).is_err());
    back.close().await;
    let _ = std::fs::remove_dir_all(&root);
}
