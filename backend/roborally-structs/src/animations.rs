use crate::position::{Direction, Position};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "server", derive(Serialize))]
#[cfg_attr(feature = "client", derive(Deserialize))]
pub enum Animation {
    /// from, to, flight direction, is_from_tank
    BulletFlight {
        from: Position,
        to: Position,
        direction: Direction,
        is_from_tank: bool,
    },
}
