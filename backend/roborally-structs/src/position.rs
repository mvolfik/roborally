use std::ops::Sub;

use serde::{Deserialize, Serialize};
#[cfg(feature = "client")]
use wasm_bindgen::prelude::wasm_bindgen;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[cfg_attr(feature = "server", derive(Serialize))]
#[cfg_attr(feature = "client", wasm_bindgen)]
pub struct Position {
    pub x: i16,
    pub y: i16,
}

impl Position {
    /// Whether the rectangle [origin, self) contains other
    #[inline]
    pub const fn contains(self, other: Self) -> bool {
        0 <= other.x && other.x < self.x && 0 <= other.y && other.y < self.y
    }

    #[inline]
    /// Returns a new position moved by one tile in given direction
    pub fn moved_in_direction(&self, dir: Direction) -> Self {
        let Position { x, y } = self;
        match dir {
            Direction::Up => Position { x: *x, y: y - 1 },
            Direction::Right => Position { x: x + 1, y: *y },
            Direction::Down => Position { x: *x, y: y + 1 },
            Direction::Left => Position { x: x - 1, y: *y },
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Deserialize)]
#[cfg_attr(feature = "server", derive(Serialize))]
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
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[cfg_attr(feature = "server", derive(Serialize))]
/// A direction that can continuously rotate by more that 270 degrees in one direction
///
/// While rotation by 360 degrees equals no rotation, CSS used to rotate robots weirdly
/// jump when you transition between 270 and 0, so this continuous direction is kept
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

    /// Returns closest [`ContinuousDirection`] that aims in the given simple [`Direction`]
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
