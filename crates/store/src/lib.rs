//! SQLite persistence for DesktopBooker: migrations, repository
//! implementations and (later) backup/restore.
//!
//! Callers use the traits in [`repo`]; [`SqliteStore`] implements them.

mod error;
pub mod repo;
mod rows;
mod seed;
mod sqlite;

pub use error::{StoreError, StoreResult};
pub use repo::{FloorRepo, RoomTypeRepo, SettingsRepo, SpaceRepo};
pub use sqlite::SqliteStore;
