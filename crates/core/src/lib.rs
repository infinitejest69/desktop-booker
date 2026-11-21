//! Pure domain logic for DesktopBooker.
//!
//! This crate must stay free of I/O (no database, files, or network) so it
//! can be unit-tested quickly and reused by every front end (desktop, LAN
//! server, Android).

pub mod grid;
pub mod ids;
pub mod property;
pub mod validation;

// Re-export the main types so callers can write `booker_core::Floor`
// instead of `booker_core::property::Floor`.
pub use grid::{GridRect, MAX_GRID_CELLS};
pub use ids::{FloorId, RoomTypeId, SpaceId};
pub use property::{Floor, PropertySettings, RoomType, Space, SpaceKind, SuiteCapacities};
pub use validation::{
    ValidationError, ValidationResult, validate_floor, validate_floor_delete, validate_room_type,
    validate_room_type_delete, validate_settings, validate_space,
};

/// Crate version, handy for an "About" screen.
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_is_set() {
        assert!(!version().is_empty());
    }
}
