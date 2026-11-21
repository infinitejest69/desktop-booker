//! Strongly-typed IDs.
//!
//! Each entity gets its own ID type wrapping a UUID (the "newtype" pattern).
//! The compiler then stops us passing a `FloorId` where a `SpaceId` is
//! expected, even though both are UUIDs underneath.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Declares a newtype ID. `macro_rules!` is Rust's pattern-based macro system;
/// it saves writing the same struct + impls once per entity.
macro_rules! define_id {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
        // `transparent` = serialise as the inner UUID string, not `{ "0": "…" }`.
        #[serde(transparent)]
        #[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
        pub struct $name(pub Uuid);

        impl $name {
            /// A new random, time-ordered (v7) ID.
            pub fn new() -> Self {
                Self(Uuid::now_v7())
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                self.0.fmt(f)
            }
        }
    };
}

define_id!(
    /// Identifies a [`crate::Floor`].
    FloorId
);
define_id!(
    /// Identifies a [`crate::Space`] (a bedroom or function suite).
    SpaceId
);
define_id!(
    /// Identifies a [`crate::RoomType`].
    RoomTypeId
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_unique_and_serialise_as_plain_strings() {
        let a = FloorId::new();
        let b = FloorId::new();
        assert_ne!(a, b);

        let json = serde_json::to_string(&a).unwrap();
        assert_eq!(json, format!("\"{a}\""));
        let back: FloorId = serde_json::from_str(&json).unwrap();
        assert_eq!(back, a);
    }
}
