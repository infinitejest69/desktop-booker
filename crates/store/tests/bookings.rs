//! Booking store tests, each against a fresh in-memory database.

use booker_core::{
    Adjustment, BoardOption, BoardOptionId, Booking, BookingId, BookingRoom, BookingRoomId,
    BookingSource, BookingStatus, DateRange, Floor, FloorId, GridRect, Guest, GuestId, Money,
    RateRule, RateRuleId, RoomType, RoomTypeId, Space, SpaceId, SpaceKind, SuiteCapacities,
    ValidationError,
};
use booker_store::{
    BookingRepo, FloorRepo, GuestRepo, RateRepo, RoomTypeRepo, SettingsRepo, SpaceRepo,
    SqliteStore, StoreError,
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
            features: Vec::new(),
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
            cars: 0,
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
            0,
        )
        .await
        .unwrap();
    assert_eq!(q.total, Money::from_major(90 + 15 + 2 * 12 + 6));
}

#[tokio::test]
async fn parking_is_priced_per_car_per_night_and_fixed_when_booked() {
    let f = fixture().await;
    let mut settings = f.store.get_settings().await.unwrap();
    settings.parking_price = Money::from_major(8);
    f.store.save_settings(&settings).await.unwrap();

    let g = guest("Hughes");
    let mut b = booking(&g, &f.rooms[0], stay("2026-05-10", "2026-05-12"));
    b.rooms[0].cars = 2;
    let saved = f.store.save_booking(&b, &g).await.unwrap();
    assert_eq!(saved.booking.rooms[0].cars, 2);
    let nights = &saved.pricing[0].nights;
    assert!(nights.iter().all(|n| n.parking == Money::from_major(16)));
    assert_eq!(
        saved.pricing[0].calculated,
        Money::from_major(2 * (90 + 16))
    );

    // A later price change doesn't touch a booking already made...
    settings.parking_price = Money::from_major(20);
    f.store.save_settings(&settings).await.unwrap();
    let again = f.store.save_booking(&saved.booking, &g).await.unwrap();
    assert_eq!(again.pricing[0].nights[0].parking, Money::from_major(16));

    // ...but changing the number of cars prices it again, at today's price.
    let mut one_car = again.booking.clone();
    one_car.rooms[0].cars = 1;
    let repriced = f.store.save_booking(&one_car, &g).await.unwrap();
    assert_eq!(repriced.pricing[0].nights[0].parking, Money::from_major(20));
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
async fn seed_demo_loads_the_lanterns_hotel() {
    use booker_core::{DEMO_SEED, FolioOwner, demo_hotel};
    use booker_store::{EventRepo, InvoiceRepo, OperationsRepo};
    let store = SqliteStore::open_in_memory().await.unwrap();
    let started = std::time::Instant::now();
    assert!(store.seed_demo().await.unwrap());
    eprintln!("demo hotel loaded in {:?}", started.elapsed());
    let today = chrono::Local::now().date_naive();
    let plan = demo_hotel(DEMO_SEED, today);

    assert_eq!(
        store.get_settings().await.unwrap().hotel_name,
        "The Lanterns Hotel"
    );
    assert_eq!(store.list_board_options().await.unwrap().len(), 3);
    assert_eq!(
        store.list_rate_rules().await.unwrap().len(),
        plan.rate_rules.len()
    );

    // Every planned stay was saved: no double bookings anywhere.
    let first = today - chrono::Days::new(booker_core::demo::HISTORY_DAYS);
    let last = today + chrono::Days::new(booker_core::demo::FUTURE_DAYS + 30);
    let chart = store.chart(first, last).await.unwrap();
    let planned_rooms: usize = plan
        .stays
        .iter()
        .filter(|s| s.booking.status.holds_rooms())
        .map(|s| s.booking.rooms.len())
        .sum();
    assert!(
        chart.len() >= planned_rooms,
        "{} of {planned_rooms}",
        chart.len()
    );
    let around_today = store
        .chart(today, today + chrono::Days::new(1))
        .await
        .unwrap();
    let statuses: std::collections::HashSet<_> = around_today.iter().map(|e| e.status).collect();
    assert!(statuses.contains(&BookingStatus::CheckedIn), "{statuses:?}");

    // Busy summers, quiet Januaries.
    let month = |y: i32, m: u32| {
        let start = NaiveDate::from_ymd_opt(y, m, 1).unwrap();
        let end = if m == 12 {
            NaiveDate::from_ymd_opt(y + 1, 1, 1).unwrap()
        } else {
            NaiveDate::from_ymd_opt(y, m + 1, 1).unwrap()
        };
        DateRange::new(start, end).unwrap()
    };
    let august = if today >= NaiveDate::from_ymd_opt(today.year(), 9, 1).unwrap() {
        month(today.year(), 8)
    } else {
        month(today.year() - 1, 8)
    };
    let january = if today >= NaiveDate::from_ymd_opt(today.year(), 2, 1).unwrap() {
        month(today.year(), 1)
    } else {
        month(today.year() - 1, 1)
    };
    let sold = |days: Vec<booker_core::DayOccupancy>| days.iter().map(|d| d.occupied).sum::<u32>();
    let aug = sold(store.occupancy_report(august).await.unwrap());
    let jan = sold(store.occupancy_report(january).await.unwrap());
    assert!(aug > jan * 2, "August {aug} vs January {jan}");

    // A stay that's over has been paid in full; many have invoices.
    let paid = plan
        .stays
        .iter()
        .find(|s| s.account.settle.is_some() && s.account.invoice_on.is_some())
        .unwrap();
    let folio = store
        .get_folio(FolioOwner::Booking(paid.booking.id))
        .await
        .unwrap();
    assert_eq!(folio.totals.balance, Money::ZERO);
    assert_eq!(folio.invoices.len(), 1);
    assert!(
        folio
            .lines
            .iter()
            .any(|l| l.charge.tax_basis_points == 2000)
    );

    // A coach tour at its group rate is paid in full by the operator.
    let tour = plan
        .stays
        .iter()
        .find(|s| s.booking.notes.contains(" tour: ") && s.account.settle.is_some())
        .unwrap();
    let folio = store
        .get_folio(FolioOwner::Booking(tour.booking.id))
        .await
        .unwrap();
    assert_eq!(folio.totals.balance, Money::ZERO);
    assert!(folio.totals.gross > Money::ZERO);

    // References follow the year the booking was made.
    let booked = store.get_booking(paid.booking.id).await.unwrap();
    assert!(
        booked
            .booking
            .reference
            .starts_with(&paid.booked_on.year().to_string()),
        "{} booked {}",
        booked.booking.reference,
        paid.booked_on
    );

    // Events, with held rooms for weddings far enough ahead.
    let events = store.list_events(NaiveDate::MIN).await.unwrap();
    assert_eq!(events.len(), plan.events.len());
    if let Some(blocked) = plan.events.iter().find(|e| e.event.room_block.is_some()) {
        let detail = store.get_event(blocked.event.id).await.unwrap();
        assert!(detail.held_rooms > 0, "the wedding holds rooms");
    }

    // Housekeeping has been round some rooms this morning.
    let hk = store.list_housekeeping(today).await.unwrap();
    assert!(
        hk.iter()
            .any(|r| r.status == booker_core::HousekeepingStatus::Dirty)
    );

    // Loading again does nothing.
    assert!(!store.seed_demo().await.unwrap());
}

#[tokio::test]
async fn check_out_dirties_the_room_and_reports_add_up() {
    use booker_core::HousekeepingStatus;
    use booker_store::OperationsRepo;
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

    // Rooms start ready; checking out leaves this one dirty.
    // Before the stay, nobody has slept in either room.
    let ready = f.store.list_housekeeping(d("2026-05-01")).await.unwrap();
    assert_eq!(ready.len(), 2);
    assert!(
        ready
            .iter()
            .all(|r| r.status == HousekeepingStatus::Inspected)
    );
    for s in [BookingStatus::CheckedIn, BookingStatus::CheckedOut] {
        f.store
            .set_booking_status(saved.booking.id, s)
            .await
            .unwrap();
    }
    // The morning after the first night, the room is dirty (slept in).
    let after = f.store.list_housekeeping(d("2026-05-11")).await.unwrap();
    let room = |id| after.iter().find(|r| r.space_id == id).unwrap().status;
    assert_eq!(room(f.rooms[0].id), HousekeepingStatus::Dirty);
    assert_eq!(room(f.rooms[1].id), HousekeepingStatus::Inspected);
    f.store
        .set_housekeeping(f.rooms[0].id, HousekeepingStatus::Clean, "")
        .await
        .unwrap();
    // Marked today, so it stays clean even though it was slept in.
    let today = chrono::Local::now().date_naive();
    let cleaned = f.store.list_housekeeping(today).await.unwrap();
    assert_eq!(
        cleaned
            .iter()
            .find(|r| r.space_id == f.rooms[0].id)
            .unwrap()
            .status,
        HousekeepingStatus::Clean
    );
    // A whole floor at once.
    assert_eq!(
        f.store
            .set_floor_housekeeping(f.rooms[0].floor_id, HousekeepingStatus::Inspected)
            .await
            .unwrap(),
        2
    );

    // Hand count: 1 of 2 rooms on the 10th and 11th, none on the 12th and 13th
    // = 2 of 8 room-nights = 25%.
    let days = f
        .store
        .occupancy_report(stay("2026-05-10", "2026-05-14"))
        .await
        .unwrap();
    let per_day: Vec<_> = days.iter().map(|d| d.per_mille).collect();
    assert_eq!(per_day, vec![500, 500, 0, 0]);
    assert_eq!(booker_core::occupancy_total(&days).per_mille, 250);

    // Two nights at £90 in May.
    let revenue = f
        .store
        .revenue_report(stay("2026-01-01", "2026-12-31"))
        .await
        .unwrap();
    assert_eq!(revenue.len(), 1);
    assert_eq!((revenue[0].month, revenue[0].rooms), (5, Money(18_000)));

    // A duplicate record for the same person, with their own booking, merges in.
    let dup = Guest {
        email: "jo.hughes@example.com".into(),
        ..guest("Hughes")
    };
    let mut theirs = booking(&dup, &f.rooms[1], stay("2026-06-01", "2026-06-02"));
    theirs.status = BookingStatus::Confirmed;
    f.store.save_booking(&theirs, &dup).await.unwrap();
    f.store.merge_guests(g.id, dup.id).await.unwrap();
    assert!(
        f.store
            .search_guests("hughes")
            .await
            .unwrap()
            .iter()
            .all(|x| x.id != dup.id),
        "the duplicate is gone"
    );
    let merged = f.store.guest_profile(g.id).await.unwrap();
    assert_eq!(merged.stays.len(), 2);
    assert_eq!(
        merged.guest.email, "hughes@example.com",
        "the kept email wins"
    );
    f.store
        .set_booking_status(theirs.id, BookingStatus::Cancelled)
        .await
        .unwrap();

    // The guest's history and spend, then erased.
    let profile = f.store.guest_profile(g.id).await.unwrap();
    // Their own stay plus the merged one (cancelled, so not in the spend).
    assert_eq!(profile.stays.len(), 2);
    assert_eq!(profile.total_spend, Money(18_000));
    f.store.erase_guest(g.id).await.unwrap();
    let erased = f.store.guest_profile(g.id).await.unwrap();
    assert_eq!(erased.guest.display_name(), "Erased guest");
    assert!(erased.guest.email.is_empty());
    assert_eq!(erased.stays.len(), 2, "bookings are kept");
}
