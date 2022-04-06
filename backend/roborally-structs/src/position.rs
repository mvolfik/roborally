use std::ops::Sub;

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
    #[inline]
    pub const fn to_continuous(&self) -> ContinuousDirection {
        use Direction::*;
        match self {
            Up => ContinuousDirection(0),
            Right => ContinuousDirection(1),
            Down => ContinuousDirection(2),
            Left => ContinuousDirection(-1),
        }
    }

    #[inline]
    pub fn rotated(&self) -> Direction {
        use Direction::*;
        match self {
            Up => Right,
            Right => Down,
            Down => Left,
            Left => Up,
        }
    }
    #[inline]
    pub fn rotated_ccw(&self) -> Direction {
        use Direction::*;
        match self {
            Up => Left,
            Left => Down,
            Down => Right,
            Right => Up,
        }
    }

    // Return position 1 tile to <Self>
    pub fn apply_to(&self, Position { x, y }: &Position) -> Position {
        match self {
            Direction::Up => Position {
                x: *x,
                y: y.wrapping_sub(1),
            },
            Direction::Right => Position { x: x + 1, y: *y },
            Direction::Down => Position { x: *x, y: y + 1 },
            Direction::Left => Position {
                x: x.wrapping_sub(1),
                y: *y,
            },
        }
    }
}

#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "server", derive(Serialize))]
#[cfg_attr(feature = "client", derive(Deserialize))]
pub struct ContinuousDirection(i64);

impl ContinuousDirection {
    pub fn to_basic(&self) -> Direction {
        let rem = self.0.rem_euclid(4);
        if rem == 0 {
            Direction::Up
        } else if rem == 1 {
            Direction::Right
        } else if rem == 2 {
            Direction::Down
        } else {
            debug_assert_eq!(rem, 3);
            Direction::Left
        }
    }
    #[inline]
    pub fn is_none(&self) -> bool {
        self.0 == 0
    }
    #[inline]
    pub fn get_rotation(&self) -> i64 {
        self.0 * 90
    }

    #[inline]
    pub fn rotated(&self) -> Self {
        Self(self.0 + 1)
    }

    #[inline]
    pub fn rotated_ccw(&self) -> Self {
        Self(self.0 - 1)
    }

    pub fn closest_in_given_basic_direction(&self, target: Direction) -> Self {
        let basic_self = self.to_basic();
        if basic_self == target {
            *self
        } else if basic_self.rotated() == target {
            self.rotated()
        } else if basic_self.rotated_ccw() == target {
            self.rotated_ccw()
        } else {
            self.rotated().rotated()
        }
    }
}

impl Sub for ContinuousDirection {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        ContinuousDirection(self.0 - rhs.0)
    }
}
