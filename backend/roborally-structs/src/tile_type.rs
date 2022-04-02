use serde::{Deserialize, Serialize};

use crate::position::Direction;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "server", derive(Serialize))]
#[cfg_attr(feature = "client", derive(Deserialize))]
pub enum BeltEnd {
    /// ''
    Straight,
    /// 'l'
    TurnLeft,
    /// 'r'
    TurnRight,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "server", derive(Serialize))]
#[cfg_attr(feature = "client", derive(Deserialize))]
pub enum TileType {
    /// `V`
    Void,
    /// `F`
    Floor,
    /// `B(f|s){dir}[end]`
    /// bool = is_fast
    Belt(bool, Direction, BeltEnd),
    /// `P{dir}[1][2][3][4][5]`
    PushPanel(Direction, [bool; 5]),
    /// `R(cw|ccw)`
    /// bool = is_clockwise
    Rotation(bool),
}

impl Default for TileType {
    fn default() -> Self {
        Self::Void
    }
}
