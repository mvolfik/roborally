#![warn(clippy::pedantic)]
#![allow(clippy::unused_unit)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::enum_glob_use)]
#![allow(clippy::many_single_char_names)]
pub mod tile;
mod utils;

use std::panic;
use wasm_bindgen::prelude::*;

use crate::tile::Tile;

///// INIT /////
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[wasm_bindgen(start)]
pub fn init_panic_hook() {
    panic::set_hook(Box::new(console_error_panic_hook::hook));
}
///// /INIT /////

#[wasm_bindgen]
pub struct GameMap {
    /// With coordinates starting at top left, index = x * width + y
    tiles: Vec<Tile>,
    width: usize,
    height: usize,
}

#[wasm_bindgen]
impl GameMap {
    #[must_use]
    pub fn get_tile(&self, x: usize, y: usize) -> Option<Tile> {
        if x >= self.width || y >= self.height {
            return None;
        }
        self.tiles.get(y * self.width + x).copied()
    }
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn width(&self) -> usize {
        self.width
    }
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn height(&self) -> usize {
        self.height
    }

    pub fn parse(s: &str) -> Result<GameMap, String> {
        let mut tiles = Vec::new();
        let mut width = None;
        let mut height = 0;
        for line in s.lines() {
            height += 1;
            let mut this_line_width = 0;
            for tile_spec in line.split(',') {
                this_line_width += 1;
                tiles.push(Tile::parse(tile_spec.trim())?);
            }
            if let Some(w) = width {
                if this_line_width != w {
                    return Err("Lines must contain same number of tiles".to_string());
                }
            } else {
                width = Some(this_line_width);
            }
        }
        Ok(GameMap {
            tiles,
            width: width.ok_or_else(|| "No tiles found".to_string())?,
            height,
        })
    }
}
