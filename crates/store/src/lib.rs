//! SQLite persistence for DesktopBooker: migrations, repository
//! implementations and (later) backup/restore.
//!
//! Callers use the traits in [`repo`]; [`SqliteStore`] implements them.

mod audit;
mod bookings;
mod error;
mod events;
pub mod repo;
mod rows;
mod seed;
mod sqlite;
mod users;

pub use error::{StoreError, StoreResult};
pub use repo::{
    BookingRepo, EventRepo, FloorRepo, GuestRepo, RateRepo, RoomTypeRepo, SettingsRepo, SpaceRepo,
    SuiteRepo,
};
pub use sqlite::{Actor, SqliteStore};
