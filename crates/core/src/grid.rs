//! Floor-plan grid geometry.
//!
//! Each floor is a grid of `cols × rows` cells. A room or suite occupies a
//! rectangle of whole cells. Coordinates start at the top-left (0, 0), and the
//! right/bottom edges are exclusive: `x..x+w`, like a Rust range.

use serde::{Deserialize, Serialize};

/// Largest allowed floor grid in either direction.
pub const MAX_GRID_CELLS: u16 = 200;

/// A rectangle of grid cells.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct GridRect {
    pub x: u16,
    pub y: u16,
    pub w: u16,
    pub h: u16,
}

impl GridRect {
    pub const fn new(x: u16, y: u16, w: u16, h: u16) -> Self {
        Self { x, y, w, h }
    }

    /// Exclusive right edge. Uses `u32` so `x + w` can't overflow a `u16`.
    pub fn right(&self) -> u32 {
        u32::from(self.x) + u32::from(self.w)
    }

    /// Exclusive bottom edge.
    pub fn bottom(&self) -> u32 {
        u32::from(self.y) + u32::from(self.h)
    }

    /// A rectangle with zero width or height covers no cells.
    pub fn is_empty(&self) -> bool {
        self.w == 0 || self.h == 0
    }

    /// True if the two rectangles share at least one cell.
    /// Touching edges (e.g. one ends at x=4, the next starts at x=4) is fine.
    pub fn overlaps(&self, other: &GridRect) -> bool {
        if self.is_empty() || other.is_empty() {
            return false;
        }
        u32::from(self.x) < other.right()
            && u32::from(other.x) < self.right()
            && u32::from(self.y) < other.bottom()
            && u32::from(other.y) < self.bottom()
    }

    /// True if the rectangle lies entirely inside a `cols × rows` grid.
    pub fn fits_within(&self, cols: u16, rows: u16) -> bool {
        self.right() <= u32::from(cols) && self.bottom() <= u32::from(rows)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const A: GridRect = GridRect::new(0, 0, 4, 3);

    #[test]
    fn identical_rects_overlap() {
        assert!(A.overlaps(&A));
    }

    #[test]
    fn partial_overlap_is_detected_both_ways() {
        let b = GridRect::new(3, 2, 2, 2);
        assert!(A.overlaps(&b));
        assert!(b.overlaps(&A));
    }

    #[test]
    fn touching_edges_do_not_overlap() {
        let right_neighbour = GridRect::new(4, 0, 2, 3);
        let below_neighbour = GridRect::new(0, 3, 4, 1);
        assert!(!A.overlaps(&right_neighbour));
        assert!(!A.overlaps(&below_neighbour));
    }

    #[test]
    fn contained_rect_overlaps() {
        let inner = GridRect::new(1, 1, 1, 1);
        assert!(A.overlaps(&inner));
        assert!(inner.overlaps(&A));
    }

    #[test]
    fn empty_rect_never_overlaps() {
        let empty = GridRect::new(1, 1, 0, 2);
        assert!(empty.is_empty());
        assert!(!A.overlaps(&empty));
    }

    #[test]
    fn fits_within_checks_exclusive_edges() {
        assert!(A.fits_within(4, 3));
        assert!(!A.fits_within(3, 3));
        assert!(!A.fits_within(4, 2));
    }

    #[test]
    fn edges_do_not_overflow_u16() {
        let r = GridRect::new(u16::MAX, u16::MAX, u16::MAX, 1);
        assert_eq!(r.right(), 2 * u32::from(u16::MAX));
        assert!(!r.fits_within(MAX_GRID_CELLS, MAX_GRID_CELLS));
    }
}
