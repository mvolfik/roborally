use crate::position::{Direction, Position};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "server", derive(Serialize))]
#[cfg_attr(feature = "client", derive(Deserialize))]
pub enum Animation {
    /// from, to, flight direction, is_from_tank
    BulletFlight(Position, Position, Direction, bool),
}
