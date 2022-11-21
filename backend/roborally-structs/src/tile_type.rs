use serde::{Deserialize, Serialize};

use crate::position::Direction;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[cfg_attr(feature = "server", derive(Serialize))]
pub enum TileType {
    /// `V`
    Void,
    /// `F`
    Floor,
    /// `B(f|s){dir}`
    /// bool = is_fast
    Belt(bool, Direction),
    /// `P{dir}{divisor}+{remainder}`
    /// Panel is active on register_i % divisor == remainder
    PushPanel(Direction, usize, usize),
    /// `R(cw|ccw)`
    /// bool = is_clockwise
    Rotation(bool),
}

impl Default for TileType {
    fn default() -> Self {
        Self::Void
    }
}
