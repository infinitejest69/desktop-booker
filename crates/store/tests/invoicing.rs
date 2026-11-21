//! Folio and invoice store tests, each against a fresh in-memory database.

use booker_core::{
    BoardOption, BoardOptionId, Booking, BookingId, BookingRoom, BookingRoomId, BookingSource,
    BookingStatus, Charge, ChargeCategory, ChargeId, DateRange, Event, EventId, EventKind, Floor,
    FloorId, FolioOwner, GridRect, Guest, GuestId, InvoiceSettings, Layout, Money, Payment,
    PaymentId, PaymentMethod, RoomType, RoomTypeId, Space, SpaceId, SpaceKind, SuiteBooking,
    SuiteBookingId, SuiteBookingStatus, SuiteCapacities, SuitePricing, SuiteRates, TaxRate,
    TaxRateId, ValidationError,
};
use booker_store::{
    BookingRepo, EventRepo, FloorRepo, InvoiceRepo, RateRepo, RoomTypeRepo, SpaceRepo, SqliteStore,
    StoreError, SuiteRepo,
};
use chrono::{NaiveDate, NaiveDateTime};

fn d(s: &str) -> NaiveDate {
    s.parse().unwrap()
}

struct Fixture {
    store: SqliteStore,
    booking: BookingId,
    guest: Guest,
    suite: SpaceId,
}

/// Room 101 (£95 a night) booked for 2 nights with B&B (£12 per adult),
/// VAT at 20% on rooms, board and suites, food at 0%.
async fn fixture() -> Fixture {
    let store = SqliteStore::open_in_memory().await.unwrap();
    let floor = Floor {
        id: FloorId::new(),
        name: "First".into(),
        level: 1,
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
        base_rate: Money::from_major(95),
        extra_adult_rate: Money::ZERO,
        child_rate: Money::ZERO,
    };
    store.save_room_type(&double).await.unwrap();
    let room = Space {
        id: SpaceId::new(),
        floor_id: floor.id,
        kind: SpaceKind::Room,
        name: "101".into(),
        room_type_id: Some(double.id),
        suite_capacities: SuiteCapacities::default(),
        rect: GridRect::new(0, 0, 3, 3),
        out_of_service: false,
        notes: String::new(),
    };
    store.save_space(&room).await.unwrap();
    let suite = Space {
        id: SpaceId::new(),
        floor_id: floor.id,
        kind: SpaceKind::Suite,
        name: "Oak Room".into(),
        room_type_id: None,
        suite_capacities: SuiteCapacities {
            banquet: Some(40),
            ..SuiteCapacities::default()
        },
        rect: GridRect::new(10, 0, 6, 4),
        out_of_service: false,
        notes: String::new(),
    };
    store.save_space(&suite).await.unwrap();
    store
        .save_suite_rates(
            suite.id,
            &SuiteRates {
                full_day: Some(Money::from_major(300)),
                setup_minutes: 0,
                cleardown_minutes: 0,
                ..SuiteRates::default()
            },
        )
        .await
        .unwrap();
    let bb = BoardOption {
        id: BoardOptionId::new(),
        code: "BB".into(),
        name: "Bed & breakfast".into(),
        adult_price: Money::from_major(12),
        child_price: Money::ZERO,
    };
    store.save_board_option(&bb).await.unwrap();

    let standard = TaxRate {
        id: TaxRateId::new(),
        name: "Standard VAT".into(),
        basis_points: 2000,
    };
    let zero = TaxRate {
        id: TaxRateId::new(),
        name: "Zero rated".into(),
        basis_points: 0,
    };
    store.save_tax_rate(&standard).await.unwrap();
    store.save_tax_rate(&zero).await.unwrap();
    store
        .save_invoice_settings(&InvoiceSettings {
            accommodation_rate: Some(standard.id),
            board_rate: Some(standard.id),
            suite_rate: Some(standard.id),
            food_rate: Some(zero.id),
            footer: "VAT no. GB 123 4567 89".into(),
            ..InvoiceSettings::default()
        })
        .await
        .unwrap();

    let guest = Guest {
        id: GuestId::new(),
        first_name: "Jo".into(),
        last_name: "Hughes".into(),
        email: String::new(),
        phone: String::new(),
        address: "1 High Street, Tenby".into(),
        notes: String::new(),
    };
    let booking = Booking {
        id: BookingId::new(),
        reference: String::new(),
        guest_id: guest.id,
        status: BookingStatus::Confirmed,
        source: BookingSource::Phone,
        notes: String::new(),
        rooms: vec![BookingRoom {
            id: BookingRoomId::new(),
            space_id: room.id,
            stay: DateRange::new(d("2026-06-19"), d("2026-06-21")).unwrap(),
            adults: 2,
            children: 0,
            board_option_id: Some(bb.id),
            price_override: None,
        }],
    };
    store.save_booking(&booking, &guest).await.unwrap();
    Fixture {
        store,
        booking: booking.id,
        guest,
        suite: suite.id,
    }
}

fn payment(pence: i64, deposit: bool) -> Payment {
    Payment {
        id: PaymentId::new(),
        date: d("2026-06-01"),
        amount: Money(pence),
        method: PaymentMethod::Card,
        is_deposit: deposit,
        reference: String::new(),
    }
}

fn extra(description: &str, category: ChargeCategory, pence: i64, rate: u32) -> Charge {
    Charge {
        id: ChargeId::new(),
        date: d("2026-06-20"),
        category,
        description: description.into(),
        quantity: 1,
        unit_price: Money(pence),
        tax_basis_points: rate,
    }
}

#[tokio::test]
async fn a_booking_folio_adds_up_its_stay_extras_and_payments() {
    let f = fixture().await;
    let owner = FolioOwner::Booking(f.booking);
    let folio = f.store.get_folio(owner).await.unwrap();
    assert!(folio.title.starts_with("Booking "));
    assert_eq!(folio.bill_to, "Jo Hughes");
    // 2 × £95 room and 2 × £24 breakfast for two, VAT inclusive at 20%.
    assert_eq!(folio.lines.len(), 2);
    assert!(folio.lines.iter().all(|l| l.automatic));
    assert_eq!(folio.totals.gross, Money(19_000 + 4_800));
    assert_eq!(folio.totals.tax, Money(3_167 + 800));

    let lunch = Charge {
        quantity: 2,
        ..extra("Sandwiches", ChargeCategory::Food, 650, 0)
    };
    let folio = f.store.add_charge(owner, &lunch).await.unwrap();
    assert_eq!(folio.lines.len(), 3);
    assert!(!folio.lines[2].automatic);
    assert_eq!(folio.totals.gross, Money(23_800 + 1_300));

    f.store
        .add_payment(owner, &payment(5_000, true))
        .await
        .unwrap();
    let folio = f
        .store
        .add_payment(owner, &payment(15_000, false))
        .await
        .unwrap();
    assert_eq!(folio.totals.deposits, Money(5_000));
    assert_eq!(folio.totals.balance, Money(25_100 - 20_000));

    // Refunding more than was paid is refused.
    let err = f
        .store
        .add_payment(owner, &payment(-30_000, false))
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        StoreError::Validation(ValidationError::RefundTooLarge)
    ));

    let folio = f.store.remove_charge(lunch.id).await.unwrap();
    assert_eq!(folio.lines.len(), 2);
}

#[tokio::test]
async fn invoices_are_numbered_in_order_and_frozen() {
    let f = fixture().await;
    let owner = FolioOwner::Booking(f.booking);
    let first = f.store.issue_invoice(owner, d("2026-06-21")).await.unwrap();
    assert_eq!(first.number, "INV-000001");
    assert_eq!(first.bill_address, "1 High Street, Tenby");
    assert_eq!(first.footer, "VAT no. GB 123 4567 89");
    assert_eq!(first.totals.gross, Money(23_800));

    // A later extra doesn't change the issued invoice, only the next one.
    let bar = extra("Bar", ChargeCategory::Drink, 1_000, 2000);
    f.store.add_charge(owner, &bar).await.unwrap();
    let second = f.store.issue_invoice(owner, d("2026-06-21")).await.unwrap();
    assert_eq!(second.number, "INV-000002");
    assert_eq!(second.totals.gross, Money(24_800));
    assert_eq!(f.store.get_invoice(first.id).await.unwrap(), first);

    let folio = f.store.get_folio(owner).await.unwrap();
    let numbers: Vec<_> = folio.invoices.iter().map(|i| i.number.as_str()).collect();
    assert_eq!(numbers, vec!["INV-000001", "INV-000002"]);
}

#[tokio::test]
async fn event_suites_are_billed_on_the_event_not_twice() {
    let f = fixture().await;
    let event = Event {
        id: EventId::new(),
        name: "Hughes dinner".into(),
        kind: EventKind::Dinner,
        organiser_id: f.guest.id,
        start_date: d("2026-07-01"),
        end_date: d("2026-07-01"),
        notes: String::new(),
        running_order: String::new(),
        sittings: Vec::new(),
        room_block: None,
    };
    f.store
        .save_event(&event, &f.guest, d("2026-06-01"))
        .await
        .unwrap();
    let at = |t: &str| NaiveDateTime::parse_from_str(t, "%Y-%m-%d %H:%M").unwrap();
    let hire = SuiteBooking {
        id: SuiteBookingId::new(),
        suite_id: f.suite,
        event_id: Some(event.id),
        guest_id: f.guest.id,
        title: "Dinner".into(),
        start: at("2026-07-01 18:00"),
        end: at("2026-07-01 23:00"),
        layout: Layout::Banquet,
        attendees: 30,
        setup_minutes: 0,
        cleardown_minutes: 0,
        pricing: SuitePricing::FullDay,
        status: SuiteBookingStatus::Confirmed,
        notes: String::new(),
    };
    f.store.save_suite_booking(&hire, &f.guest).await.unwrap();

    let event_folio = f
        .store
        .get_folio(FolioOwner::Event(event.id))
        .await
        .unwrap();
    assert_eq!(event_folio.lines.len(), 1);
    assert_eq!(event_folio.lines[0].charge.category, ChargeCategory::Suite);
    assert_eq!(event_folio.totals.gross, Money(30_000));
    let hire_folio = f
        .store
        .get_folio(FolioOwner::SuiteBooking(hire.id))
        .await
        .unwrap();
    assert!(hire_folio.lines.is_empty());
    // Nothing to bill: no invoice, and no number used up.
    assert!(
        f.store
            .issue_invoice(FolioOwner::SuiteBooking(hire.id), d("2026-07-02"))
            .await
            .is_err()
    );
    let inv = f
        .store
        .issue_invoice(FolioOwner::Event(event.id), d("2026-07-02"))
        .await
        .unwrap();
    assert_eq!(inv.number, "INV-000001");
}
