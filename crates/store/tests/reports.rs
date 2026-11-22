//! The report builder against a real (in-memory) database: saved reports and
//! dashboards, and figures that agree with the standard reports.

use booker_core::{
    ChartKind, Dashboard, DashboardId, DashboardItem, DateRange, Measure, Money, Period,
    ReportSpec, SavedReport, SavedReportId, Split, ValidationError, occupancy_total, run_report,
};
use booker_store::{OperationsRepo, ReportRepo, SqliteStore, StoreError};
use chrono::{Datelike, Local, NaiveDate};

async fn store() -> SqliteStore {
    SqliteStore::open_in_memory()
        .await
        .expect("open in-memory db")
}

fn report(name: &str, measure: Measure, split: Split) -> SavedReport {
    SavedReport {
        id: SavedReportId::new(),
        spec: ReportSpec {
            name: name.into(),
            measure,
            split,
            period: Period::LastMonth,
            chart: ChartKind::Bar,
        },
    }
}

#[tokio::test]
async fn saved_reports_round_trip_and_bad_ones_are_refused() {
    let s = store().await;
    let revenue = report(
        "Revenue by room type",
        Measure::RoomRevenue,
        Split::RoomType,
    );
    let arrivals = report("arrivals by month", Measure::Arrivals, Split::Month);
    s.save_saved_report(&revenue).await.unwrap();
    s.save_saved_report(&arrivals).await.unwrap();
    // Listed by name, ignoring case.
    assert_eq!(
        s.list_saved_reports().await.unwrap(),
        vec![arrivals.clone(), revenue.clone()]
    );

    let mut renamed = revenue.clone();
    renamed.spec.name = "Room revenue".into();
    s.save_saved_report(&renamed).await.unwrap();
    assert_eq!(s.list_saved_reports().await.unwrap()[1], renamed);

    let bad = report("Occupancy by source", Measure::Occupancy, Split::Source);
    assert!(matches!(
        s.save_saved_report(&bad).await,
        Err(StoreError::Validation(
            ValidationError::ReportSplitNotAllowed { .. }
        ))
    ));
}

#[tokio::test]
async fn dashboards_keep_their_order_and_lose_deleted_reports() {
    let s = store().await;
    let a = report("A", Measure::Arrivals, Split::None);
    let b = report("B", Measure::Departures, Split::None);
    s.save_saved_report(&a).await.unwrap();
    s.save_saved_report(&b).await.unwrap();
    let dash = Dashboard {
        id: DashboardId::new(),
        name: "Front desk".into(),
        items: vec![
            DashboardItem {
                report_id: b.id,
                wide: true,
            },
            DashboardItem {
                report_id: a.id,
                wide: false,
            },
        ],
    };
    s.save_dashboard(&dash).await.unwrap();
    assert_eq!(s.list_dashboards().await.unwrap(), vec![dash.clone()]);

    // Deleting a report takes it off the dashboard.
    s.delete_saved_report(b.id).await.unwrap();
    let left = &s.list_dashboards().await.unwrap()[0];
    assert_eq!(
        left.items,
        vec![DashboardItem {
            report_id: a.id,
            wide: false
        }]
    );

    // A dashboard can't hold a report that doesn't exist.
    let mut ghost = dash.clone();
    ghost.items = vec![DashboardItem {
        report_id: SavedReportId::new(),
        wide: false,
    }];
    assert!(matches!(
        s.save_dashboard(&ghost).await,
        Err(StoreError::NotFound { .. })
    ));

    s.delete_dashboard(dash.id).await.unwrap();
    assert!(s.list_dashboards().await.unwrap().is_empty());
}

/// Acceptance: the builder agrees with the standard reports on the demo hotel.
#[tokio::test]
async fn figures_match_the_standard_reports() {
    let s = store().await;
    s.seed_demo().await.unwrap();
    let today = Local::now().date_naive();
    let month = NaiveDate::from_ymd_opt(today.year(), today.month(), 1).unwrap();
    let period = Period::ThisMonth.resolve(today);
    assert_eq!(period.start, month);
    let facts = s.report_facts(period).await.unwrap();
    let run = |measure| {
        run_report(
            &ReportSpec {
                name: "Check".into(),
                measure,
                split: Split::None,
                period: Period::ThisMonth,
                chart: ChartKind::Number,
            },
            &facts,
            today,
        )
        .total
    };

    let days = s.occupancy_report(period).await.unwrap();
    let occupancy = occupancy_total(&days);
    assert!(
        occupancy.occupied > 0,
        "the demo hotel has guests this month"
    );
    assert_eq!(run(Measure::RoomNightsSold), i64::from(occupancy.occupied));
    assert_eq!(run(Measure::Occupancy), i64::from(occupancy.per_mille));

    let year = DateRange {
        start: period.start,
        end: period.end,
    };
    let revenue = s.revenue_report(year).await.unwrap();
    let total: Money = revenue.iter().map(|m| m.total).sum();
    let rooms: Money = revenue.iter().map(|m| m.rooms).sum();
    let board: Money = revenue.iter().map(|m| m.board).sum();
    assert_eq!(run(Measure::TotalRevenue), total.0);
    assert_eq!(run(Measure::RoomRevenue), rooms.0);
    assert_eq!(run(Measure::BoardRevenue), board.0);
}
