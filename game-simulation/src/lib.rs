#![warn(clippy::nursery)]
#![allow(clippy::use_self)]
#![warn(clippy::pedantic)]
#![allow(clippy::unused_unit)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::enum_glob_use)]
#![allow(clippy::many_single_char_names)]
#![warn(clippy::shadow_reuse)]
#![warn(clippy::shadow_same)]
#![warn(clippy::shadow_unrelated)]
#![feature(pattern)]
#![feature(const_precise_live_drops)]
mod parser;
pub mod store;
pub mod tile;
mod utils;

use std::{convert::Into, panic};
use tile::{AssetArray, Direction, TileGrid};

use wasm_bindgen::prelude::*;

use crate::{
    parser::Parse,
    tile::{Asset, Tile, Transform},
};

///// INIT /////
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[wasm_bindgen(start)]
pub fn init_panic_hook() {
    panic::set_hook(Box::new(console_error_panic_hook::hook));
}
///// /INIT /////

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct Position {
    x: u32,
    y: u32,
}

impl Position {
    /// Whether the rectangle [origin, self) contains other
    #[inline]
    const fn contains(self, other: Self) -> bool {
        other.x < self.x && other.y < self.y
    }
}

#[wasm_bindgen]
pub struct GameMap {
    /// With coordinates starting at top left, index = x * width + y
    tiles: TileGrid,
    size: Position,
    antenna: Position,
    reboot_token: (Position, Direction),
    checkpoints: Vec<Position>,
    spawn_points: Vec<(Position, Direction)>,
}

#[wasm_bindgen]
impl GameMap {
    #[must_use]
    fn get_tile(&self, x: u32, y: u32) -> Option<&Tile> {
        self.tiles.get_tile(
            self.size.x as usize,
            self.size.y as usize,
            x as usize,
            y as usize,
        )
    }

    #[allow(clippy::missing_const_for_fn)]
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn width(&self) -> u32 {
        self.size.x
    }
    #[allow(clippy::missing_const_for_fn)]
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn height(&self) -> u32 {
        self.size.y
    }

    #[must_use]
    pub fn get_assets_at(&self, x: u32, y: u32) -> Option<AssetArray> {
        let mut assets = self.get_tile(x, y)?.get_assets();

        let this_pos = Position { x, y };
        if self.antenna == this_pos {
            assets.push(Asset {
                uri: "antenna.png".to_string(),
                transform: Transform::default(),
            });
        }
        if self.reboot_token.0 == this_pos {
            assets.push(Asset {
                uri: "reboot-token.png".to_string(),
                transform: Transform {
                    rotate: self.reboot_token.1.get_rotation(),
                    ..Transform::default()
                },
            });
        }
        if let Some((i, _)) = self
            .checkpoints
            .iter()
            .enumerate()
            .find(|(_, pos)| pos == &&this_pos)
        {
            assets.push(Asset {
                uri: "checkpoint.png".to_string(),
                transform: Transform::default(),
            });
            assets.push(Asset {
                uri: format!("number-{}.png", i + 1),
                transform: Transform {
                    translate: Some((30.0, 30.0)),
                    ..Transform::default()
                },
            });
        }
        if let Some((_i, (_, dir))) = self
            .spawn_points
            .iter()
            .enumerate()
            .find(|(_, (pos, _))| pos == &this_pos)
        {
            assets.push(Asset {
                uri: "spawn-point.png".to_string(),
                transform: Transform {
                    rotate: dir.get_rotation(),
                    ..Transform::default()
                },
            });
        }
        Some(assets.into())
    }

    /// First line is a header:
    /// header : {prop}( {prop})*
    /// prop   : Size={pos} | Antenna={pos} | Reboot={pos}:{dir} | Checkpoints=[{pos}];+ | Spawnpoints=[{pos}:{dir}];+
    /// pos    : <x>,<y>
    /// dir    : u | r | d | l
    ///
    /// Then follow Size.y remaining lines
    pub fn parse(s: &str) -> Result<GameMap, String> {
        <GameMap as Parse>::parse(s, "Map").map_err(parser::ParseError::get)
    }
}
