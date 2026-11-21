//! SQLite persistence for DesktopBooker: migrations, repository
//! implementations and (later) backup/restore.
//!
//! Callers use the traits in [`repo`]; [`SqliteStore`] implements them.

mod audit;
mod backups;
mod bookings;
mod email;
mod error;
mod events;
mod invoicing;
mod operations;
pub mod repo;
mod rows;
mod seed;
mod sqlite;
mod users;

pub use backups::{list_backups, restore_file};
pub use error::{StoreError, StoreResult};
pub use repo::{
    BackupRepo, BookingRepo, EmailRepo, EventRepo, FloorRepo, GuestRepo, InvoiceRepo,
    OperationsRepo, RateRepo, RoomTypeRepo, SettingsRepo, SpaceRepo, SuiteRepo,
};
pub use sqlite::{Actor, SqliteStore};
