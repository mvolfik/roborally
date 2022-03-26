use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "server", derive(Serialize))]
#[cfg_attr(feature = "client", derive(Deserialize))]
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
}
