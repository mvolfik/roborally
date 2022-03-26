use serde::{Deserialize, Serialize};

use crate::{
    position::{Direction, Position},
    tile::{Grid, Tile},
};

#[derive(Clone, Debug)]
#[cfg_attr(feature = "server", derive(Serialize))]
#[cfg_attr(feature = "client", derive(Deserialize))]
pub struct GameMap {
    pub tiles: Grid<Tile>,
    pub antenna: Position,
    pub reboot_token: (Position, Direction),
    pub checkpoints: Vec<Position>,
    pub spawn_points: Vec<(Position, Direction)>,
}
