use crate::{
    position::{Direction, Position},
    tile_type::TileType,
};
use serde::{Deserialize, Serialize};

#[allow(clippy::struct_excessive_bools)]
#[derive(Default, Clone, Copy, Debug, PartialEq, Eq, Hash, Deserialize)]
#[cfg_attr(feature = "server", derive(Serialize))]
pub struct DirectionBools {
    pub up: bool,
    pub right: bool,
    pub down: bool,
    pub left: bool,
}

impl DirectionBools {
    #[must_use]
    pub const fn get(&self, dir: Direction) -> bool {
        match dir {
            Direction::Up => self.up,
            Direction::Right => self.right,
            Direction::Down => self.down,
            Direction::Left => self.left,
        }
    }

    #[must_use]
    pub const fn to_items(&self) -> [(Direction, bool); 4] {
        [
            (Direction::Up, self.up),
            (Direction::Right, self.right),
            (Direction::Down, self.down),
            (Direction::Left, self.left),
        ]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[cfg_attr(feature = "server", derive(Serialize))]
pub struct Tile {
    pub typ: TileType,
    pub walls: DirectionBools,
}

#[derive(Clone, Debug, Deserialize)]
#[cfg_attr(feature = "server", derive(Serialize))]
pub struct Grid<T> {
    vec: Vec<T>,
    size: Position,
}

impl<T> Grid<T> {
    #[must_use]
    pub fn get(&self, pos: Position) -> Option<&T> {
        if 0 > pos.x || pos.x >= self.size.x || 0 > pos.y || pos.y >= self.size.y {
            return None;
        }
        self.vec.get((pos.y * self.size.x + pos.x) as usize)
    }

    #[must_use]
    pub fn get_mut(&mut self, pos: Position) -> Option<&mut T> {
        if 0 > pos.x || pos.x >= self.size.x || 0 > pos.y || pos.y >= self.size.y {
            return None;
        }
        self.vec.get_mut((pos.y * self.size.x + pos.x) as usize)
    }

    #[must_use]
    pub const fn size(&self) -> Position {
        self.size
    }

    #[must_use]
    pub const fn vec(&self) -> &Vec<T> {
        &self.vec
    }

    pub fn new(vec: Vec<T>, size: Position) -> Result<Self, String> {
        if (size.x * size.y) as usize == vec.len() {
            Ok(Self { vec, size })
        } else {
            Err("Supplied position doesn't match vector size".to_owned())
        }
    }
}
