use std::{
    cmp::Ordering,
    ops::{Add, Sub},
};

use serde::{Deserialize, Serialize};
#[cfg(feature = "client")]
use wasm_bindgen::prelude::wasm_bindgen;
/// (0,0) &rarr; +x  \
/// &darr;  \
/// +y
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
    #[must_use]
    pub const fn contains(self, other: Self) -> bool {
        0 <= other.x && other.x < self.x && 0 <= other.y && other.y < self.y
    }

    #[inline]
    #[must_use]
    /// Returns a new position moved by one tile in given direction
    pub const fn moved_in_direction(&self, dir: Direction) -> Self {
        let Position { x, y } = *self;
        match dir {
            Direction::Up => Position { x, y: y - 1 },
            Direction::Right => Position { x: x + 1, y },
            Direction::Down => Position { x, y: y + 1 },
            Direction::Left => Position { x: x - 1, y },
        }
    }
}

/// Small utility type that helps sorting players by priority
#[derive(PartialEq, Eq, Clone, Copy)]
pub struct Priority {
    x_diff: i16,
    y_diff: i16,
}

impl Priority {
    #[must_use]
    pub fn new(player: Position, antenna: Position) -> Self {
        Self {
            x_diff: player.x - antenna.x,
            y_diff: player.y - antenna.y,
        }
    }

    fn dist(self) -> u16 {
        self.x_diff.unsigned_abs() + self.y_diff.unsigned_abs()
    }

    /// Whatever this calculates, the result is bearing from antenna to player, starting at North == 0 and increasing clockwise
    fn bearing(self) -> f64 {
        f64::from(self.x_diff)
            .atan2(f64::from(-self.y_diff))
            .rem_euclid(std::f64::consts::TAU)
    }
}

impl Ord for Priority {
    fn cmp(&self, other: &Self) -> Ordering {
        if self.eq(other) {
            return Ordering::Equal;
        }
        self.dist()
            .cmp(&other.dist())
            .then_with(|| self.bearing().partial_cmp(&other.bearing()).unwrap())
    }
}

impl PartialOrd for Priority {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Deserialize)]
#[cfg_attr(feature = "server", derive(Serialize))]
#[cfg_attr(feature = "client", wasm_bindgen)]
pub enum Direction {
    Up,
    Right,
    Down,
    Left,
}

impl Direction {
    #[inline]
    #[must_use]
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
    #[must_use]
    pub const fn rotated(&self) -> Direction {
        use Direction::*;
        match self {
            Up => Right,
            Right => Down,
            Down => Left,
            Left => Up,
        }
    }
    #[inline]
    #[must_use]
    pub const fn rotated_ccw(&self) -> Direction {
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
    #[inline]
    #[must_use]
    pub const fn is_none(&self) -> bool {
        self.0 == 0
    }

    #[inline]
    #[must_use]
    pub const fn get_rotation(&self) -> i64 {
        self.0 * 90
    }

    #[inline]
    #[must_use]
    pub const fn rotated(&self) -> Self {
        Self(self.0 + 1)
    }

    #[inline]
    #[must_use]
    pub const fn rotated_ccw(&self) -> Self {
        Self(self.0 - 1)
    }

    #[must_use]
    /// Returns closest [`ContinuousDirection`] that aims in the given simple [`Direction`]
    pub fn closest_in_given_basic_direction(&self, target: Direction) -> Self {
        let basic_self: Direction = (*self).into();
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

impl From<ContinuousDirection> for Direction {
    fn from(dir: ContinuousDirection) -> Self {
        let rem = dir.0.rem_euclid(4);
        if rem == 0 {
            Direction::Up
        } else if rem == 1 {
            Direction::Right
        } else if rem == 2 {
            Direction::Down
        } else {
            Direction::Left
        }
    }
}

impl From<ContinuousDirection> for i64 {
    fn from(dir: ContinuousDirection) -> Self {
        dir.0
    }
}

impl<T: Into<i64>> Sub<T> for ContinuousDirection {
    type Output = Self;

    fn sub(self, rhs: T) -> Self::Output {
        ContinuousDirection(self.0 - rhs.into())
    }
}

impl<T: Into<i64>> Add<T> for ContinuousDirection {
    type Output = Self;

    fn add(self, rhs: T) -> Self::Output {
        ContinuousDirection(self.0 + rhs.into())
    }
}

impl PartialEq for ContinuousDirection {
    fn eq(&self, other: &Self) -> bool {
        self.0.rem_euclid(4) == other.0.rem_euclid(4)
    }
}
