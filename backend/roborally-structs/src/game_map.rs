use serde::{Deserialize, Serialize};

use crate::{
    position::{Direction, Position},
    tile::{Grid, Tile},
};

#[derive(Clone)]
#[cfg_attr(feature = "server", derive(Serialize))]
#[cfg_attr(feature = "client", derive(Deserialize))]
pub struct GameMap {
    pub tiles: Grid<Tile>,
    pub antenna: Position,
    pub reboot_token: (Position, Direction),
    pub checkpoints: Vec<Position>,
    pub spawn_points: Vec<(Position, Direction)>,
    pub lasers: Vec<(Position, Direction)>,
}

impl std::fmt::Debug for GameMap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GameMap").finish_non_exhaustive()
    }
}
