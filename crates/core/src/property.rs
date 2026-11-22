//! The hotel's physical setup: settings, floors, room types and spaces.

use serde::{Deserialize, Serialize};

use crate::grid::GridRect;
use crate::ids::{FloorId, RoomTypeId, SpaceId};
use crate::money::Money;

/// Hotel-wide settings (there is exactly one of these).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct PropertySettings {
    pub hotel_name: String,
    pub address: String,
    pub phone: String,
    pub email: String,
    /// ISO 4217 code, e.g. "GBP", "EUR".
    pub currency: String,
    /// IANA time zone, e.g. "Europe/London".
    pub timezone: String,
    /// Usual arrival time, "HH:MM".
    pub check_in_time: String,
    /// Usual departure time, "HH:MM".
    pub check_out_time: String,
    /// Lock the app after this many idle minutes (0 = never).
    pub idle_lock_minutes: u16,
    /// Parking, per car per night. Zero until the hotel sets a price.
    pub parking_price: Money,
}

impl Default for PropertySettings {
    fn default() -> Self {
        Self {
            hotel_name: "My Hotel".into(),
            address: String::new(),
            phone: String::new(),
            email: String::new(),
            currency: "GBP".into(),
            timezone: "Europe/London".into(),
            check_in_time: "15:00".into(),
            check_out_time: "11:00".into(),
            idle_lock_minutes: 15,
            parking_price: Money::ZERO,
        }
    }
}

/// One floor of the building, drawn as a grid.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct Floor {
    pub id: FloorId,
    /// Display name, e.g. "Ground floor".
    pub name: String,
    /// Building level: 0 = ground, 1 = first, -1 = basement. Floors are listed by level.
    pub level: i32,
    pub grid_cols: u16,
    pub grid_rows: u16,
}

/// A category of bedroom with shared pricing and occupancy, e.g. "Double".
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct RoomType {
    pub id: RoomTypeId,
    pub name: String,
    /// Short code shown on the tariff chart, e.g. "DBL".
    pub code: String,
    /// Guests normally staying (used for default pricing later).
    pub default_occupancy: u8,
    pub max_occupancy: u8,
    pub description: String,
    /// Price per room per night before any rate rules.
    pub base_rate: Money,
    /// Per night, for each adult above `default_occupancy`.
    pub extra_adult_rate: Money,
    /// Per night, for each child.
    pub child_rate: Money,
}

/// Whether a space is a bedroom or a function suite.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub enum SpaceKind {
    Room,
    Suite,
}

/// Something a bedroom offers (or allows) that guests may ask for.
/// A room without `Smoking` is non-smoking.
///
/// `Ord` follows the order written here, which is also the order they're shown in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub enum RoomFeature {
    Wifi,
    PetsAllowed,
    /// Accessible for wheelchair users (step-free, accessible bathroom).
    Accessible,
    Smoking,
}

impl RoomFeature {
    pub const ALL: [RoomFeature; 4] = [
        RoomFeature::Wifi,
        RoomFeature::PetsAllowed,
        RoomFeature::Accessible,
        RoomFeature::Smoking,
    ];

    /// The name stored in the database (the same as the JSON name).
    pub const fn as_str(self) -> &'static str {
        match self {
            RoomFeature::Wifi => "wifi",
            RoomFeature::PetsAllowed => "pets_allowed",
            RoomFeature::Accessible => "accessible",
            RoomFeature::Smoking => "smoking",
        }
    }
}

/// Features as stored in one text column: `"wifi,pets_allowed"`, in a fixed
/// order and without repeats, so the same room always saves the same text.
pub fn features_to_text(features: &[RoomFeature]) -> String {
    let mut list = features.to_vec();
    list.sort();
    list.dedup(); // removes repeats, which `sort` has put next to each other
    list.iter()
        .map(|f| f.as_str())
        .collect::<Vec<_>>()
        .join(",")
}

/// Reads [`features_to_text`]'s text back. An unknown name is returned as the error.
pub fn features_from_text(text: &str) -> Result<Vec<RoomFeature>, String> {
    let mut list = text
        .split(',')
        .filter(|part| !part.is_empty())
        .map(|part| {
            RoomFeature::ALL
                .into_iter()
                .find(|f| f.as_str() == part)
                .ok_or_else(|| part.to_string())
        })
        // Collecting into `Result<Vec<_>, _>` stops at the first `Err`.
        .collect::<Result<Vec<_>, _>>()?;
    list.sort();
    list.dedup();
    Ok(list)
}

/// Seated or standing capacity of a function suite for each layout.
/// `None` means the suite isn't offered in that layout.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct SuiteCapacities {
    pub theatre: Option<u16>,
    pub boardroom: Option<u16>,
    pub banquet: Option<u16>,
    pub cabaret: Option<u16>,
    pub reception: Option<u16>,
}

impl SuiteCapacities {
    /// The largest capacity across all layouts (0 if none are set).
    pub fn max(&self) -> u16 {
        [
            self.theatre,
            self.boardroom,
            self.banquet,
            self.cabaret,
            self.reception,
        ]
        .into_iter()
        .flatten()
        .max()
        .unwrap_or(0)
    }
}

/// Something bookable that occupies a rectangle on a floor plan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct Space {
    pub id: SpaceId,
    pub floor_id: FloorId,
    pub kind: SpaceKind,
    /// Room number ("101") or suite name ("Garden Suite"). Unique across the hotel.
    pub name: String,
    /// Required for rooms; must be `None` for suites.
    pub room_type_id: Option<RoomTypeId>,
    /// Only meaningful for suites.
    pub suite_capacities: SuiteCapacities,
    pub rect: GridRect,
    /// Temporarily unavailable (maintenance etc.).
    pub out_of_service: bool,
    /// What a bedroom offers; always empty for suites.
    pub features: Vec<RoomFeature>,
    pub notes: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suite_max_capacity_ignores_unset_layouts() {
        let caps = SuiteCapacities {
            theatre: Some(120),
            banquet: Some(80),
            ..Default::default()
        };
        assert_eq!(caps.max(), 120);
        assert_eq!(SuiteCapacities::default().max(), 0);
    }

    #[test]
    fn room_features_are_stored_as_text_sorted_without_repeats() {
        let features = [
            RoomFeature::Smoking,
            RoomFeature::Wifi,
            RoomFeature::Smoking,
        ];
        assert_eq!(features_to_text(&features), "wifi,smoking");
        assert_eq!(features_to_text(&[]), "");
    }

    #[test]
    fn room_features_read_back_from_text() {
        assert_eq!(
            features_from_text("pets_allowed,wifi"),
            Ok(vec![RoomFeature::Wifi, RoomFeature::PetsAllowed])
        );
        assert_eq!(features_from_text(""), Ok(vec![]));
        assert_eq!(
            features_from_text("wifi,jacuzzi"),
            Err("jacuzzi".to_string())
        );
    }

    #[test]
    fn every_feature_round_trips() {
        let text = features_to_text(&RoomFeature::ALL);
        assert_eq!(features_from_text(&text), Ok(RoomFeature::ALL.to_vec()));
    }

    #[test]
    fn room_feature_serialises_snake_case() {
        assert_eq!(
            serde_json::to_string(&RoomFeature::PetsAllowed).unwrap(),
            "\"pets_allowed\""
        );
    }

    #[test]
    fn space_kind_serialises_lowercase() {
        assert_eq!(
            serde_json::to_string(&SpaceKind::Suite).unwrap(),
            "\"suite\""
        );
    }
}
