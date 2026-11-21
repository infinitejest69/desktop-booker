//! Booking store tests, each against a fresh in-memory database.

use booker_core::{
    Adjustment, BoardOption, BoardOptionId, Booking, BookingId, BookingRoom, BookingRoomId,
    BookingSource, BookingStatus, DateRange, Floor, FloorId, GridRect, Guest, GuestId, Money,
    RateRule, RateRuleId, RoomType, RoomTypeId, Space, SpaceId, SpaceKind, SuiteCapacities,
    ValidationError,
};
use booker_store::{
    BookingRepo, FloorRepo, GuestRepo, RateRepo, RoomTypeRepo, SpaceRepo, SqliteStore, StoreError,
};
use chrono::{Datelike, NaiveDate};

fn d(s: &str) -> NaiveDate {
    s.parse().unwrap()
}

fn stay(a: &str, b: &str) -> DateRange {
    DateRange::new(d(a), d(b)).unwrap()
}

struct Fixture {
    store: SqliteStore,
    rooms: Vec<Space>,
}

/// One floor, a Double room type at £90 (+£25 extra adult, +£15 child), rooms 101 and 102.
async fn fixture() -> Fixture {
    let store = SqliteStore::open_in_memory().await.unwrap();
    let floor = Floor {
        id: FloorId::new(),
        name: "First".into(),
        level: 1,
        grid_cols: 20,
        grid_rows: 10,
    };
    store.save_floor(&floor).await.unwrap();
    let room_type = RoomType {
        id: RoomTypeId::new(),
        name: "Double".into(),
        code: "DBL".into(),
        default_occupancy: 2,
        max_occupancy: 3,
        description: String::new(),
        base_rate: Money::from_major(90),
        extra_adult_rate: Money::from_major(25),
        child_rate: Money::from_major(15),
    };
    store.save_room_type(&room_type).await.unwrap();
    let mut rooms = Vec::new();
    for (i, name) in ["101", "102"].iter().enumerate() {
        let space = Space {
            id: SpaceId::new(),
            floor_id: floor.id,
            kind: SpaceKind::Room,
            name: (*name).into(),
            room_type_id: Some(room_type.id),
            suite_capacities: SuiteCapacities::default(),
            rect: GridRect::new(u16::try_from(i).unwrap() * 3, 0, 3, 3),
            out_of_service: false,
            notes: String::new(),
        };
        store.save_space(&space).await.unwrap();
        rooms.push(space);
    }
    Fixture { store, rooms }
}

fn guest(last: &str) -> Guest {
    Guest {
        id: GuestId::new(),
        first_name: "Jo".into(),
        last_name: last.into(),
        email: format!("{}@example.com", last.to_lowercase()),
        phone: String::new(),
        address: String::new(),
        notes: String::new(),
    }
}

fn booking(guest: &Guest, space: &Space, s: DateRange) -> Booking {
    Booking {
        id: BookingId::new(),
        reference: String::new(),
        guest_id: guest.id,
        status: BookingStatus::Confirmed,
        source: BookingSource::Phone,
        notes: String::new(),
        rooms: vec![BookingRoom {
            id: BookingRoomId::new(),
            space_id: space.id,
            stay: s,
            adults: 2,
            children: 0,
            board_option_id: None,
            price_override: None,
        }],
    }
}

#[tokio::test]
async fn a_new_booking_gets_a_reference_and_priced_nights() {
    let f = fixture().await;
    let g = guest("Hughes");
    let b = booking(&g, &f.rooms[0], stay("2026-05-10", "2026-05-13"));
    let saved = f.store.save_booking(&b, &g).await.unwrap();

    let year = chrono::Local::now().year();
    assert_eq!(saved.booking.reference, format!("{year}-0001"));
    assert_eq!(saved.pricing[0].nights.len(), 3);
    assert_eq!(saved.total, Money::from_major(270));

    let b2 = booking(&g, &f.rooms[1], stay("2026-05-10", "2026-05-11"));
    let second = f.store.save_booking(&b2, &g).await.unwrap();
    assert_eq!(second.booking.reference, format!("{year}-0002"));
}

#[tokio::test]
async fn double_booking_is_refused_but_same_day_turnover_is_fine() {
    let f = fixture().await;
    let (a, b) = (guest("Hughes"), guest("Price"));
    let first = booking(&a, &f.rooms[0], stay("2026-05-10", "2026-05-13"));
    f.store.save_booking(&first, &a).await.unwrap();

    let clash = booking(&b, &f.rooms[0], stay("2026-05-12", "2026-05-14"));
    let err = f.store.save_booking(&clash, &b).await.unwrap_err();
    assert!(
        matches!(
            err,
            StoreError::Validation(ValidationError::RoomUnavailable { .. })
        ),
        "{err:?}"
    );
    // The failed save leaves nothing behind, not even the new guest.
    assert!(f.store.search_guests("price").await.unwrap().is_empty());

    let turnover = booking(&b, &f.rooms[0], stay("2026-05-13", "2026-05-14"));
    f.store.save_booking(&turnover, &b).await.unwrap();
}

#[tokio::test]
async fn cancelling_frees_the_room_and_reinstating_rechecks_it() {
    let f = fixture().await;
    let (a, b) = (guest("Hughes"), guest("Price"));
    let first = f
        .store
        .save_booking(
            &booking(&a, &f.rooms[0], stay("2026-05-10", "2026-05-12")),
            &a,
        )
        .await
        .unwrap();

    let cancelled = f
        .store
        .set_booking_status(first.booking.id, BookingStatus::Cancelled)
        .await
        .unwrap();
    assert_eq!(
        cancelled.total,
        Money::ZERO,
        "cancelled bookings hold no priced nights"
    );
    f.store
        .save_booking(
            &booking(&b, &f.rooms[0], stay("2026-05-10", "2026-05-12")),
            &b,
        )
        .await
        .unwrap();

    let err = f
        .store
        .set_booking_status(first.booking.id, BookingStatus::Confirmed)
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        StoreError::Validation(ValidationError::RoomUnavailable { .. })
    ));
}

#[tokio::test]
async fn status_changes_follow_the_rules() {
    let f = fixture().await;
    let g = guest("Hughes");
    let saved = f
        .store
        .save_booking(
            &booking(&g, &f.rooms[0], stay("2026-05-10", "2026-05-12")),
            &g,
        )
        .await
        .unwrap();
    let id = saved.booking.id;

    let err = f
        .store
        .set_booking_status(id, BookingStatus::CheckedOut)
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        StoreError::Validation(ValidationError::InvalidStatusChange { .. })
    ));
    f.store
        .set_booking_status(id, BookingStatus::CheckedIn)
        .await
        .unwrap();
    let out = f
        .store
        .set_booking_status(id, BookingStatus::CheckedOut)
        .await
        .unwrap();
    assert_eq!(out.booking.status, BookingStatus::CheckedOut);
}

#[tokio::test]
async fn saved_prices_survive_rate_changes_until_the_stay_changes() {
    let f = fixture().await;
    let g = guest("Hughes");
    let saved = f
        .store
        .save_booking(
            &booking(&g, &f.rooms[0], stay("2026-05-10", "2026-05-12")),
            &g,
        )
        .await
        .unwrap();
    assert_eq!(saved.total, Money::from_major(180));

    // Prices go up after the booking was made.
    f.store
        .save_rate_rule(&RateRule {
            id: RateRuleId::new(),
            name: "Rise".into(),
            room_type_id: None,
            start: d("2026-01-01"),
            end: d("2026-12-31"),
            weekdays: vec![],
            priority: 0,
            adjustment: Adjustment::SetPrice {
                amount: Money::from_major(150),
            },
            active: true,
        })
        .await
        .unwrap();

    // Checking in keeps the original prices.
    let checked_in = f
        .store
        .set_booking_status(saved.booking.id, BookingStatus::CheckedIn)
        .await
        .unwrap();
    assert_eq!(checked_in.total, Money::from_major(180));

    // Extending the stay re-prices that room at today's rates.
    let mut longer = checked_in.booking.clone();
    longer.rooms[0].stay = stay("2026-05-10", "2026-05-13");
    let repriced = f.store.save_booking(&longer, &g).await.unwrap();
    assert_eq!(repriced.total, Money::from_major(450));
}

#[tokio::test]
async fn override_replaces_the_room_total() {
    let f = fixture().await;
    let g = guest("Hughes");
    let mut b = booking(&g, &f.rooms[0], stay("2026-05-10", "2026-05-12"));
    b.rooms[0].price_override = Some(Money::from_major(150));
    let saved = f.store.save_booking(&b, &g).await.unwrap();
    assert_eq!(saved.pricing[0].calculated, Money::from_major(180));
    assert_eq!(saved.total, Money::from_major(150));
}

#[tokio::test]
async fn quotes_include_board_and_supplements() {
    let f = fixture().await;
    let bb = BoardOption {
        id: BoardOptionId::new(),
        code: "bb".into(),
        name: "Bed & breakfast".into(),
        adult_price: Money::from_major(12),
        child_price: Money::from_major(6),
    };
    f.store.save_board_option(&bb).await.unwrap();
    assert_eq!(f.store.list_board_options().await.unwrap()[0].code, "BB");

    let q = f
        .store
        .quote_room(
            f.rooms[0].id,
            stay("2026-05-10", "2026-05-11"),
            2,
            1,
            Some(bb.id),
        )
        .await
        .unwrap();
    assert_eq!(q.total, Money::from_major(90 + 15 + 2 * 12 + 6));
}

#[tokio::test]
async fn chart_free_rooms_and_search() {
    let f = fixture().await;
    let g = guest("Hughes");
    let saved = f
        .store
        .save_booking(
            &booking(&g, &f.rooms[0], stay("2026-05-10", "2026-05-12")),
            &g,
        )
        .await
        .unwrap();

    let chart = f
        .store
        .chart(d("2026-05-01"), d("2026-05-31"))
        .await
        .unwrap();
    assert_eq!(chart.len(), 1);
    assert_eq!(chart[0].guest_name, "Jo Hughes");
    assert!(
        f.store
            .chart(d("2026-06-01"), d("2026-06-30"))
            .await
            .unwrap()
            .is_empty()
    );

    let free = f
        .store
        .free_rooms(stay("2026-05-11", "2026-05-12"), None)
        .await
        .unwrap();
    assert_eq!(free, vec![f.rooms[1].id]);
    let free = f
        .store
        .free_rooms(stay("2026-05-11", "2026-05-12"), Some(saved.booking.id))
        .await
        .unwrap();
    assert_eq!(free.len(), 2);

    let found = f.store.search_bookings("hugh").await.unwrap();
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].rooms, vec!["101".to_string()]);
    assert_eq!(
        f.store
            .search_bookings(&saved.booking.reference)
            .await
            .unwrap()
            .len(),
        1
    );
    assert!(f.store.search_bookings("nobody").await.unwrap().is_empty());
}

#[tokio::test]
async fn chart_and_search_show_each_rooms_board() {
    let f = fixture().await;
    let bb = BoardOption {
        id: BoardOptionId::new(),
        code: "BB".into(),
        name: "Bed & breakfast".into(),
        adult_price: Money::from_major(12),
        child_price: Money::from_major(6),
    };
    f.store.save_board_option(&bb).await.unwrap();
    let g = guest("Hughes");
    let mut b = booking(&g, &f.rooms[0], stay("2026-05-10", "2026-05-12"));
    b.rooms[0].board_option_id = Some(bb.id);
    let mut second = b.rooms[0].clone();
    second.id = BookingRoomId::new();
    second.space_id = f.rooms[1].id;
    second.board_option_id = None;
    b.rooms.push(second);
    f.store.save_booking(&b, &g).await.unwrap();

    let chart = f
        .store
        .chart(d("2026-05-01"), d("2026-05-31"))
        .await
        .unwrap();
    let board_of = |room: &Space| {
        chart
            .iter()
            .find(|e| e.space_id == room.id)
            .and_then(|e| e.board.clone())
    };
    assert_eq!(board_of(&f.rooms[0]).as_deref(), Some("Bed & breakfast"));
    assert_eq!(board_of(&f.rooms[1]), None);

    let found = f.store.search_bookings("hughes").await.unwrap();
    let pairs: Vec<_> = found[0]
        .rooms
        .iter()
        .cloned()
        .zip(found[0].boards.iter().cloned())
        .collect();
    assert!(pairs.contains(&("101".into(), Some("Bed & breakfast".into()))));
    assert!(pairs.contains(&("102".into(), None)));
}

#[tokio::test]
async fn seed_demo_includes_rates_and_bookings() {
    let store = SqliteStore::open_in_memory().await.unwrap();
    store.seed_demo().await.unwrap();
    assert_eq!(store.list_board_options().await.unwrap().len(), 3);
    assert_eq!(store.list_rate_rules().await.unwrap().len(), 4);
    let today = chrono::Local::now().date_naive();
    let from = today - chrono::Days::new(14);
    let chart = store
        .chart(from, today + chrono::Days::new(46))
        .await
        .unwrap();
    assert!(chart.len() >= 40, "a busy demo hotel, got {}", chart.len());
    let statuses: std::collections::HashSet<_> = chart.iter().map(|e| e.status).collect();
    for s in [
        BookingStatus::CheckedOut,
        BookingStatus::CheckedIn,
        BookingStatus::Confirmed,
    ] {
        assert!(statuses.contains(&s), "demo has {s:?} bookings");
    }
    assert!(store.search_bookings("").await.unwrap().len() >= 20);
    // Demo events: a wedding with suites, sittings and held rooms.
    use booker_store::EventRepo;
    let events = store.list_events(today).await.unwrap();
    assert_eq!(events.len(), 3);
    let wedding = events.iter().find(|e| e.name.contains("wedding")).unwrap();
    let detail = store.get_event(wedding.id).await.unwrap();
    assert_eq!(detail.suites.len(), 2);
    assert_eq!(detail.event.sittings.len(), 2);
    assert!(detail.held_rooms > 0, "the wedding holds some rooms");
    // Seeding events again does nothing.
    assert!(!store.seed_demo_events().await.unwrap());
    // Demo stays come with a mix of boards, including room only.
    let boards: std::collections::HashSet<_> = chart.iter().map(|e| e.board.clone()).collect();
    assert!(boards.contains(&Some("Bed & breakfast".into())));
    assert!(boards.contains(&None));
}
