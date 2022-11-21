use serde::{Deserialize, Serialize};

use crate::{
    position::{Direction, Position},
    tile::{Grid, Tile},
};

#[derive(Clone, Deserialize)]
#[cfg_attr(feature = "server", derive(Serialize))]
pub struct GameMap {
    pub name: String,
    pub tiles: Grid<Tile>,
    pub antenna: Position,
    pub reboot_token: (Position, Direction),
    pub checkpoints: Vec<Position>,
    pub spawn_points: Vec<(Position, Direction)>,
    pub lasers: Vec<(Position, Direction)>,
}

impl std::fmt::Debug for GameMap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let size = self.tiles.size();
        f.debug_struct("GameMap")
            .field("width", &size.x)
            .field("height", &size.y)
            .finish_non_exhaustive()
    }
}
