//! Suite hire and event store tests, each against a fresh in-memory database.

use booker_core::{
    BookingStatus, CateringSitting, DateRange, Event, EventId, EventKind, Floor, FloorId, GridRect,
    Guest, GuestId, Layout, Money, RoomBlock, RoomType, RoomTypeId, Space, SpaceId, SpaceKind,
    SuiteBooking, SuiteBookingId, SuiteBookingStatus, SuiteCapacities, SuitePricing, SuiteRates,
    ValidationError,
};
use booker_store::{
    BookingRepo, EventRepo, FloorRepo, RoomTypeRepo, SpaceRepo, SqliteStore, StoreError, SuiteRepo,
};
use chrono::{NaiveDate, NaiveDateTime, NaiveTime};

fn d(s: &str) -> NaiveDate {
    s.parse().unwrap()
}

fn t(s: &str) -> NaiveDateTime {
    NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M").unwrap()
}

struct Fixture {
    store: SqliteStore,
    suite: Space,
    rooms: Vec<Space>,
    double: RoomTypeId,
}

/// A suite (theatre 100, banquet 60) at £50/hour (min 3h) or £400/day,
/// with 60 min setup and 30 min clear-down, plus three double bedrooms.
async fn fixture() -> Fixture {
    let store = SqliteStore::open_in_memory().await.unwrap();
    let floor = Floor {
        id: FloorId::new(),
        name: "Ground".into(),
        level: 0,
        grid_cols: 30,
        grid_rows: 10,
    };
    store.save_floor(&floor).await.unwrap();
    let double = RoomType {
        id: RoomTypeId::new(),
        name: "Double".into(),
        code: "DBL".into(),
        default_occupancy: 2,
        max_occupancy: 2,
        description: String::new(),
        base_rate: Money::from_major(100),
        extra_adult_rate: Money::ZERO,
        child_rate: Money::ZERO,
    };
    store.save_room_type(&double).await.unwrap();
    let suite = Space {
        id: SpaceId::new(),
        floor_id: floor.id,
        kind: SpaceKind::Suite,
        name: "Garden Suite".into(),
        room_type_id: None,
        suite_capacities: SuiteCapacities {
            theatre: Some(100),
            banquet: Some(60),
            ..SuiteCapacities::default()
        },
        rect: GridRect::new(0, 0, 8, 4),
        out_of_service: false,
        notes: String::new(),
    };
    store.save_space(&suite).await.unwrap();
    store
        .save_suite_rates(
            suite.id,
            &SuiteRates {
                hourly: Some(Money::from_major(50)),
                min_hours: 3,
                half_day: None,
                full_day: Some(Money::from_major(400)),
                setup_minutes: 60,
                cleardown_minutes: 30,
            },
        )
        .await
        .unwrap();
    let mut rooms = Vec::new();
    for (i, name) in ["101", "102", "103"].iter().enumerate() {
        let room = Space {
            id: SpaceId::new(),
            floor_id: floor.id,
            kind: SpaceKind::Room,
            name: (*name).into(),
            room_type_id: Some(double.id),
            suite_capacities: SuiteCapacities::default(),
            rect: GridRect::new(10 + u16::try_from(i).unwrap() * 3, 0, 3, 3),
            out_of_service: false,
            notes: String::new(),
        };
        store.save_space(&room).await.unwrap();
        rooms.push(room);
    }
    Fixture {
        store,
        suite,
        rooms,
        double: double.id,
    }
}

fn organiser() -> Guest {
    Guest {
        id: GuestId::new(),
        first_name: "Ffion".into(),
        last_name: "Hughes".into(),
        email: "ffion@example.com".into(),
        phone: String::new(),
        address: String::new(),
        notes: String::new(),
    }
}

fn hire(f: &Fixture, guest: &Guest, title: &str, start: &str, end: &str) -> SuiteBooking {
    SuiteBooking {
        id: SuiteBookingId::new(),
        suite_id: f.suite.id,
        event_id: None,
        guest_id: guest.id,
        title: title.into(),
        start: t(start),
        end: t(end),
        layout: Layout::Theatre,
        attendees: 40,
        setup_minutes: 60,
        cleardown_minutes: 30,
        pricing: SuitePricing::Hourly,
        status: SuiteBookingStatus::Confirmed,
        notes: String::new(),
    }
}

#[tokio::test]
async fn suite_rates_round_trip() {
    let f = fixture().await;
    let rates = f.store.get_suite_rates(f.suite.id).await.unwrap();
    assert_eq!(rates.hourly, Some(Money::from_major(50)));
    assert_eq!(rates.min_hours, 3);
    assert_eq!(rates.cleardown_minutes, 30);
    // Bedrooms have no rate card.
    assert!(matches!(
        f.store.get_suite_rates(f.rooms[0].id).await,
        Err(StoreError::NotFound { .. })
    ));
}

#[tokio::test]
async fn suite_bookings_are_priced_and_buffers_prevent_clashes() {
    let f = fixture().await;
    let g = organiser();
    // Two hours, charged at the three-hour minimum.
    let meeting = hire(
        &f,
        &g,
        "Board meeting",
        "2026-06-10 10:00",
        "2026-06-10 12:00",
    );
    let saved = f.store.save_suite_booking(&meeting, &g).await.unwrap();
    assert_eq!(saved.price, Money::from_major(150));
    assert_eq!(saved.suite_name, "Garden Suite");
    assert_eq!(saved.guest_name, "Ffion Hughes");

    // The bookings don't overlap, but the meeting's clear-down (to 12:30)
    // overlaps lunch's setup (from 11:45).
    let lunch = hire(&f, &g, "Lunch", "2026-06-10 12:45", "2026-06-10 14:00");
    let err = f.store.save_suite_booking(&lunch, &g).await.unwrap_err();
    assert!(matches!(
        err,
        StoreError::Validation(ValidationError::SuiteUnavailable { .. })
    ));

    // Clear of both buffers: clear-down ends 12:30, setup starts 12:30.
    let later = hire(&f, &g, "Lunch", "2026-06-10 13:30", "2026-06-10 15:00");
    f.store.save_suite_booking(&later, &g).await.unwrap();

    let day = f
        .store
        .list_suite_bookings(t("2026-06-10 00:00"), t("2026-06-11 00:00"))
        .await
        .unwrap();
    assert_eq!(day.len(), 2);
    assert_eq!(day[0].booking.title, "Board meeting");
}

#[tokio::test]
async fn cancelling_frees_the_suite_and_prices_stick_until_terms_change() {
    let f = fixture().await;
    let g = organiser();
    let mut meeting = hire(
        &f,
        &g,
        "Board meeting",
        "2026-06-10 10:00",
        "2026-06-10 14:00",
    );
    assert_eq!(
        f.store
            .save_suite_booking(&meeting, &g)
            .await
            .unwrap()
            .price,
        Money::from_major(200)
    );

    // A new hourly rate doesn't touch the saved price…
    let mut rates = f.store.get_suite_rates(f.suite.id).await.unwrap();
    rates.hourly = Some(Money::from_major(80));
    f.store.save_suite_rates(f.suite.id, &rates).await.unwrap();
    meeting.title = "Board meeting (AGM)".into();
    assert_eq!(
        f.store
            .save_suite_booking(&meeting, &g)
            .await
            .unwrap()
            .price,
        Money::from_major(200)
    );
    // …until the times change.
    meeting.end = t("2026-06-10 15:00");
    assert_eq!(
        f.store
            .save_suite_booking(&meeting, &g)
            .await
            .unwrap()
            .price,
        Money::from_major(400)
    );

    let cancelled = f
        .store
        .set_suite_booking_status(meeting.id, SuiteBookingStatus::Cancelled)
        .await
        .unwrap();
    assert_eq!(cancelled.booking.status, SuiteBookingStatus::Cancelled);
    let other = hire(&f, &g, "Wake", "2026-06-10 11:00", "2026-06-10 13:00");
    f.store.save_suite_booking(&other, &g).await.unwrap();

    // Reinstating re-checks availability.
    let err = f
        .store
        .set_suite_booking_status(meeting.id, SuiteBookingStatus::Confirmed)
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        StoreError::Validation(ValidationError::SuiteUnavailable { .. })
    ));
}

fn wedding(f: &Fixture, g: &Guest, rooms: u16) -> Event {
    Event {
        id: EventId::new(),
        name: "Hughes–Price wedding".into(),
        kind: EventKind::Wedding,
        organiser_id: g.id,
        start_date: d("2026-06-20"),
        end_date: d("2026-06-20"),
        notes: String::new(),
        running_order: "14:00 ceremony\n15:30 breakfast".into(),
        sittings: vec![CateringSitting {
            name: "Wedding breakfast".into(),
            date: d("2026-06-20"),
            time: NaiveTime::from_hms_opt(15, 30, 0).unwrap(),
            adults: 50,
            children: 4,
            dietary: "3 vegetarian".into(),
        }],
        room_block: Some(RoomBlock {
            room_type_id: Some(f.double),
            rooms,
            stay: DateRange::new(d("2026-06-20"), d("2026-06-22")).unwrap(),
            rate: Money::from_major(85),
            release_date: d("2026-05-20"),
        }),
    }
}

#[tokio::test]
async fn an_event_holds_its_room_block_and_totals_its_value() {
    let f = fixture().await;
    let g = organiser();
    let event = wedding(&f, &g, 2);
    let saved = f
        .store
        .save_event(&event, &g, d("2026-03-01"))
        .await
        .unwrap();
    assert_eq!(saved.event.sittings.len(), 1);
    assert_eq!(saved.catering.adults, 50);
    assert_eq!(saved.rooms.len(), 2);
    assert_eq!(saved.held_rooms, 2);
    assert!(
        saved
            .rooms
            .iter()
            .all(|r| r.status == BookingStatus::Provisional)
    );
    // Two nights at the £85 group rate per room.
    assert_eq!(saved.rooms_value, Money::from_major(2 * 170));

    let mut breakfast = hire(
        &f,
        &g,
        "Wedding breakfast",
        "2026-06-20 15:00",
        "2026-06-20 23:00",
    );
    breakfast.event_id = Some(event.id);
    breakfast.layout = Layout::Banquet;
    breakfast.attendees = 54;
    breakfast.pricing = SuitePricing::FullDay;
    f.store.save_suite_booking(&breakfast, &g).await.unwrap();

    let detail = f.store.get_event(event.id).await.unwrap();
    assert_eq!(detail.suites.len(), 1);
    assert_eq!(
        detail.suites[0].event_name.as_deref(),
        Some("Hughes–Price wedding")
    );
    assert_eq!(detail.suites_value, Money::from_major(400));
    assert_eq!(detail.total, Money::from_major(400 + 340));

    let listed = f.store.list_events(d("2026-06-01")).await.unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].suite_bookings, 1);
    assert_eq!(listed[0].held_rooms, 2);
    assert_eq!(listed[0].release_date, Some(d("2026-05-20")));
    assert!(
        f.store
            .list_events(d("2026-07-01"))
            .await
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn resizing_the_block_keeps_taken_up_rooms() {
    let f = fixture().await;
    let g = organiser();
    let mut event = wedding(&f, &g, 3);
    let saved = f
        .store
        .save_event(&event, &g, d("2026-03-01"))
        .await
        .unwrap();
    // A guest takes up the first room.
    let taken = saved.rooms[0].booking_id;
    f.store
        .set_booking_status(taken, BookingStatus::Confirmed)
        .await
        .unwrap();

    if let Some(b) = event.room_block.as_mut() {
        b.rooms = 1;
    }
    let shrunk = f
        .store
        .save_event(&event, &g, d("2026-03-01"))
        .await
        .unwrap();
    let live: Vec<_> = shrunk
        .rooms
        .iter()
        .filter(|r| r.status.holds_rooms())
        .collect();
    assert_eq!(live.len(), 1);
    assert_eq!(live[0].booking_id, taken);
    assert_eq!(shrunk.held_rooms, 0);

    // Asking for more than are free fails and changes nothing.
    if let Some(b) = event.room_block.as_mut() {
        b.rooms = 5;
    }
    let err = f
        .store
        .save_event(&event, &g, d("2026-03-01"))
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        StoreError::Validation(ValidationError::NotEnoughRooms { wanted: 4, free: 2 })
    ));
    let unchanged = f.store.get_event(event.id).await.unwrap();
    assert_eq!(unchanged.event.room_block.map(|b| b.rooms), Some(1));
}

#[tokio::test]
async fn release_day_frees_only_held_rooms() {
    let f = fixture().await;
    let g = organiser();
    let event = wedding(&f, &g, 3);
    let saved = f
        .store
        .save_event(&event, &g, d("2026-03-01"))
        .await
        .unwrap();
    f.store
        .set_booking_status(saved.rooms[0].booking_id, BookingStatus::Confirmed)
        .await
        .unwrap();

    assert_eq!(
        f.store.release_due_blocks(d("2026-05-19")).await.unwrap(),
        0
    );
    assert_eq!(
        f.store.release_due_blocks(d("2026-05-20")).await.unwrap(),
        2
    );
    // Running again finds nothing more to do.
    assert_eq!(
        f.store.release_due_blocks(d("2026-05-21")).await.unwrap(),
        0
    );

    let after = f.store.get_event(event.id).await.unwrap();
    assert_eq!(after.held_rooms, 0);
    assert_eq!(
        after
            .rooms
            .iter()
            .filter(|r| r.status == BookingStatus::Cancelled)
            .count(),
        2
    );
    // The released rooms are on sale again.
    let free = f
        .store
        .free_rooms(
            DateRange::new(d("2026-06-20"), d("2026-06-22")).unwrap(),
            None,
        )
        .await
        .unwrap();
    assert_eq!(free.len(), 2);
}
