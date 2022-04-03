use crate::{
    position::{Direction, Position},
    tile_type::TileType,
};
use serde::{Deserialize, Serialize};

#[allow(clippy::struct_excessive_bools)]
#[derive(Default, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "server", derive(Serialize))]
#[cfg_attr(feature = "client", derive(Deserialize))]
pub struct DirectionBools {
    pub up: bool,
    pub right: bool,
    pub down: bool,
    pub left: bool,
}

impl DirectionBools {
    pub fn get(&self, dir: &Direction) -> bool {
        match dir {
            Direction::Up => self.up,
            Direction::Right => self.right,
            Direction::Down => self.down,
            Direction::Left => self.left,
        }
    }

    pub fn to_items(&self) -> [(Direction, bool); 4] {
        [
            (Direction::Up, self.up),
            (Direction::Right, self.right),
            (Direction::Down, self.down),
            (Direction::Left, self.left),
        ]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "server", derive(Serialize))]
#[cfg_attr(feature = "client", derive(Deserialize))]
pub struct Tile {
    pub typ: TileType,
    pub walls: DirectionBools,
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "server", derive(Serialize))]
#[cfg_attr(feature = "client", derive(Deserialize))]
pub struct Grid<T> {
    vec: Vec<T>,
    size: Position,
}

impl<T> Grid<T> {
    pub fn get(&self, pos: Position) -> Option<&T> {
        if pos.x >= self.size.x || pos.y >= self.size.y {
            return None;
        }
        self.vec.get(pos.y * self.size.x + pos.x)
    }

    pub fn get_mut(&mut self, pos: Position) -> Option<&mut T> {
        if pos.x >= self.size.x || pos.y >= self.size.y {
            return None;
        }
        self.vec.get_mut(pos.y * self.size.x + pos.x)
    }

    pub fn size(&self) -> Position {
        self.size
    }

    pub fn vec(&self) -> &Vec<T> {
        &self.vec
    }

    pub fn new(vec: Vec<T>, size: Position) -> Result<Self, String> {
        if size.x * size.y == vec.len() {
            Ok(Self { vec, size })
        } else {
            Err("Supplied position doesn't match vector size".to_owned())
        }
    }
}

// impl<'a, T> IntoIterator for &'a Grid<T> {
//     type Item = (Position, &'a T);

//     type IntoIter = Map<Enumerate<Iter<'a, T>>, impl FnMut((usize, &T)) -> (Position, &T)>;

//     fn into_iter(self: &'a Grid<T>) -> Self::IntoIter {
//         let vec: Enumerate<Iter<'_, T>> = self.vec.iter().enumerate();
//         let f = &mut |(i, item)| {
//             (
//                 Position {
//                     x: i % self.size.x,
//                     y: i / self.size.x,
//                 },
//                 item,
//             )
//         };
//         vec.map(f)
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
