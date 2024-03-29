use crate::position::{Direction, Position};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "server", derive(Serialize))]
#[cfg_attr(feature = "client", derive(Deserialize))]
pub enum Animation {
    BulletFlight {
        from: Position,
        to: Position,
        direction: Direction,
        is_from_tank: bool,
    },
    CheckpointVisited {
        player_i: usize,
    },
    AttemptedMove {
        player_i: usize,
        direction: Direction,
    },
}
