use crate::{
    position::{Direction, Position},
    tile_type::TileType,
};
use serde::{Deserialize, Serialize};

#[allow(clippy::struct_excessive_bools)]
#[derive(Default, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "server", derive(Serialize))]
#[cfg_attr(feature = "client", derive(Deserialize))]
pub struct WallsDescription {
    pub up: bool,
    pub right: bool,
    pub down: bool,
    pub left: bool,
}

impl WallsDescription {
    pub fn get(&self, dir: &Direction) -> bool {
        match dir {
            Direction::Up => self.up,
            Direction::Right => self.right,
            Direction::Down => self.down,
            Direction::Left => self.left,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "server", derive(Serialize))]
#[cfg_attr(feature = "client", derive(Deserialize))]
pub struct Tile {
    pub typ: TileType,
    pub walls: WallsDescription,
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "server", derive(Serialize))]
#[cfg_attr(feature = "client", derive(Deserialize))]
pub struct Grid<T> {
    vec: Vec<T>,
    size: Position,
}

impl<T> Grid<T> {
    pub fn get(&self, x: usize, y: usize) -> Option<&T> {
        if x >= self.size.x || y >= self.size.y {
            return None;
        }
        self.vec.get(y * self.size.x + x)
    }

    pub fn get_mut(&mut self, x: usize, y: usize) -> Option<&mut T> {
        if x >= self.size.x || y >= self.size.y {
            return None;
        }
        self.vec.get_mut(y * self.size.x + x)
    }

    pub fn size(&self) -> Position {
        self.size
    }

    pub fn new(vec: Vec<T>, size: Position) -> Result<Self, String> {
        if size.x * size.y == vec.len() {
            Ok(Self { vec, size })
        } else {
            Err("Supplied position doesn't match vector size".to_owned())
        }
    }

    pub fn map<U>(&self, c: impl FnMut(&T) -> U) -> Grid<U> {
        Grid {
            vec: self.vec.iter().map(c).collect(),
            size: self.size,
        }
    }
}

// impl<'a, T> IntoIterator for &'a Grid<T> {
//     type Item = (Position, &'a T);

//     type IntoIter = GridIter<'a, T>;

//     fn into_iter(self) -> Self::IntoIter {
//         GridIter { grid: self, i: 0 }
//     }
// }

// pub struct GridIter<'a, T> {
//     grid: &'a Grid<T>,
//     i: usize,
// }

// impl<'a, T> Iterator for GridIter<'a, T> {
//     type Item = (Position, &'a T);

//     fn next(&mut self) -> Option<Self::Item> {
//         let size = self.grid.size();
//         let pos = Position {
//             x: self.i % size.x,
//             y: self.i / size.y,
//         };
//         let item = self.grid.get(pos.x, pos.y);
//         self.i += 1;
//         item.map(|x| (pos, x))
//     }
// }
