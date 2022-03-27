use serde::{Deserialize, Serialize};
#[cfg(feature = "client")]
use wasm_bindgen::prelude::wasm_bindgen;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "server", derive(Serialize))]
#[cfg_attr(feature = "client", derive(Deserialize), wasm_bindgen)]
pub struct Position {
    pub x: usize,
    pub y: usize,
}

impl Position {
    /// Whether the rectangle [origin, self) contains other
    #[inline]
    pub const fn contains(self, other: Self) -> bool {
        other.x < self.x && other.y < self.y
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "server", derive(Serialize))]
#[cfg_attr(feature = "client", derive(Deserialize))]
pub enum Direction {
    Up,
    Right,
    Down,
    Left,
}

impl Direction {
    /// By default, all directed tiles should point up
    #[must_use]
    pub const fn get_rotation(self) -> Option<f64> {
        use Direction::*;
        match self {
            Up => None,
            Right => Some(90.0),
            Down => Some(180.0),
            Left => Some(-90.0),
        }
    }

    pub fn rotated(&self) -> Direction {
        use Direction::*;
        match self {
            Up => Right,
            Right => Down,
            Down => Left,
            Left => Up,
        }
    }

    // Return position 1 tile to <Self>
    pub fn apply(&self, Position { x, y }: &Position) -> Position {
        match self {
            Direction::Up => Position { x: *x, y: y - 1 },
            Direction::Right => Position { x: x + 1, y: *y },
            Direction::Down => Position { x: *x, y: y + 1 },
            Direction::Left => Position { x: x - 1, y: *y },
        }
    }
}
