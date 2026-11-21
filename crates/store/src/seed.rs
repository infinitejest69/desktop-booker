//! Demo data so there is something to look at during development.

use booker_core::{
    Floor, FloorId, GridRect, RoomType, RoomTypeId, Space, SpaceId, SpaceKind, SuiteCapacities,
};

use crate::repo::{FloorRepo, RoomTypeRepo, SpaceRepo};
use crate::{SqliteStore, StoreResult};

const COLS: u16 = 24;
const ROWS: u16 = 10;
/// Rooms are 3×4 cells: a row of 8 along the top (y=0) and bottom (y=6),
/// with a corridor (y=4..6) between them.
const ROOM_W: u16 = 3;
const ROOM_H: u16 = 4;

impl SqliteStore {
    /// Fills an empty database with a 3-floor demo hotel (40 rooms, 2 suites).
    /// Returns `false` and does nothing if any floors already exist.
    pub async fn seed_demo(&self) -> StoreResult<bool> {
        if !self.list_floors().await?.is_empty() {
            return Ok(false);
        }

        let types = [
            ("Single", "SGL", 1, 1),
            ("Double", "DBL", 2, 2),
            ("Twin", "TWN", 2, 2),
            ("Family", "FAM", 3, 4),
        ]
        .map(|(name, code, default_occupancy, max_occupancy)| RoomType {
            id: RoomTypeId::new(),
            name: name.into(),
            code: code.into(),
            default_occupancy,
            max_occupancy,
            description: String::new(),
        });
        for rt in &types {
            self.save_room_type(rt).await?;
        }

        let room = |floor: &Floor, number: String, slot: u16, top: bool, type_ix: usize| Space {
            id: SpaceId::new(),
            floor_id: floor.id,
            kind: SpaceKind::Room,
            name: number,
            room_type_id: Some(types[type_ix % types.len()].id),
            suite_capacities: SuiteCapacities::default(),
            rect: GridRect::new(
                slot * ROOM_W,
                if top { 0 } else { ROWS - ROOM_H },
                ROOM_W,
                ROOM_H,
            ),
            out_of_service: false,
            notes: String::new(),
        };

        for (level, name) in [(0, "Ground floor"), (1, "First floor"), (2, "Second floor")] {
            let floor = Floor {
                id: FloorId::new(),
                name: name.into(),
                level,
                grid_cols: COLS,
                grid_rows: ROWS,
            };
            self.save_floor(&floor).await?;

            // Ground floor: 8 rooms on top, two function suites below.
            // Upper floors: 16 rooms each, numbered 101–116, 201–216.
            let slots_per_side = COLS / ROOM_W;
            let mut number: u16 = 1;
            let sides: &[bool] = if level == 0 { &[true] } else { &[true, false] };
            for &top in sides {
                for slot in 0..slots_per_side {
                    let label = if level == 0 {
                        format!("G{number:02}")
                    } else {
                        format!("{level}{number:02}")
                    };
                    self.save_space(&room(&floor, label, slot, top, usize::from(number)))
                        .await?;
                    number += 1;
                }
            }
            if level == 0 {
                for (name, x, w, caps) in [
                    (
                        "Garden Suite",
                        0,
                        14,
                        SuiteCapacities {
                            theatre: Some(150),
                            banquet: Some(110),
                            cabaret: Some(80),
                            reception: Some(200),
                            ..Default::default()
                        },
                    ),
                    (
                        "Oak Room",
                        14,
                        10,
                        SuiteCapacities {
                            theatre: Some(40),
                            boardroom: Some(20),
                            banquet: Some(30),
                            ..Default::default()
                        },
                    ),
                ] {
                    self.save_space(&Space {
                        id: SpaceId::new(),
                        floor_id: floor.id,
                        kind: SpaceKind::Suite,
                        name: name.into(),
                        room_type_id: None,
                        suite_capacities: caps,
                        rect: GridRect::new(x, ROWS - ROOM_H, w, ROOM_H),
                        out_of_service: false,
                        notes: String::new(),
                    })
                    .await?;
                }
            }
        }
        Ok(true)
    }
}
