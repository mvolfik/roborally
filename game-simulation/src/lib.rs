#![warn(clippy::nursery)]
#![allow(clippy::use_self)]
#![warn(clippy::pedantic)]
#![allow(clippy::unused_unit)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::enum_glob_use)]
#![allow(clippy::many_single_char_names)]
pub mod store;
pub mod tile;
mod utils;

use js_sys::{Object, Reflect};
use std::{
    collections::HashMap,
    convert::{Into},
    panic,
    str::FromStr,
};
use utils::StringArray;
use wasm_bindgen::{prelude::*, JsCast};

use crate::tile::Tile;

///// INIT /////
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[wasm_bindgen(start)]
pub fn init_panic_hook() {
    panic::set_hook(Box::new(console_error_panic_hook::hook));
}
///// /INIT /////

struct Position {
    x: u32,
    y: u32,
}

impl Position {
    fn parse(it: &str) -> Result<Self, String> {
        let mut split = it.split(',');
        if let (Some(x_str), Some(y_str), None) = (split.next(), split.next(), split.next()) {
            Ok(Self {
                x: u32::from_str(x_str)
                    .map_err(|_e| "Expected a number (x coordinate)".to_string())?,
                y: u32::from_str(y_str)
                    .map_err(|_e| "Expected a number (x coordinate)".to_string())?,
            })
        } else {
            Err(format!("Value `{}` doesn't have format `x,y`", it))
        }
    }
}

#[wasm_bindgen]
pub struct GameMap {
    /// With coordinates starting at top left, index = x * width + y
    tiles: Vec<Tile>,
    width: u32,
    height: u32,
}

#[wasm_bindgen(typescript_custom_section)]
const TS_SECTION_GAMEMAP_PARSE_OK_RESULT: &'static str =
    "export type GameMapParseOkResult = { warnings?: Array<string>; map: GameMap; };";

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "GameMapParseOkResult")]
    pub type GameMapParseOkResult;
}

#[wasm_bindgen]
impl GameMap {
    #[must_use]
    pub fn get_tile(&self, x: u32, y: u32) -> Option<Tile> {
        if x >= self.width || y >= self.height {
            return None;
        }
        self.tiles.get((y * self.width + x) as usize).copied()
    }
    #[allow(clippy::missing_const_for_fn)]
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn width(&self) -> u32 {
        self.width
    }
    #[allow(clippy::missing_const_for_fn)]
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn height(&self) -> u32 {
        self.height
    }

    /// First line is a header:
    /// Size={width},{height} Antenna={x},{y}
    ///
    /// Then follow {width} remaining lines
    pub fn parse(s: &str) -> Result<GameMapParseOkResult, String> {
        let mut warnings = Vec::new();
        let mut lines = s.lines();
        let first_line = &mut lines
            .next()
            .ok_or_else(|| "No lines in input".to_string())?;

        let mut props = HashMap::new();
        for propdef in first_line.split(' ') {
            let mut split = propdef.split('=');
            if let (Some(name), Some(value), None) = (split.next(), split.next(), split.next()) {
                props.insert(name, value);
            } else {
                return Err(format!(
                    "Prop definition `{}` doesn't follow format key=value",
                    propdef
                ));
            }
        }
        let Position {
            x: width,
            y: height,
        } = Position::parse(
            props
                .remove("Size")
                .ok_or_else(|| "Must specify 'Size' in header".to_string())?,
        )
        .map_err(|e| format!("Error parsing value for Size: {}", e))?;

        for name in props.keys() {
            warnings.push(format!("Unused prop in header: {}", name));
        }

        let mut tiles = Vec::new();
        let mut y = 0;
        for line in lines {
            let mut x = 0;
            for tile_spec in line.split(',') {
                tiles.push(
                    Tile::parse(tile_spec.trim())
                        .map_err(|e| format!("Parsing error on tile {},{}: {}", x, y, e))?,
                );
                x += 1;
            }

            if x != width {
                return Err(format!(
                    "Line {} contains wrong amount of tiles (found {}, expected width={})",
                    y, x, width
                ));
            }
            y += 1;
        }
        if y != height {
            return Err(format!(
                "Wrong amount of tile lines (found {}, expected height={})",
                y, height
            ));
        }
        let map = Self {
            tiles,
            width,
            height,
        };
        let object = js_object! {
            &"map".into() => &map.into(),
            &"warnings".into() =>
                &if warnings.is_empty() {
                    JsValue::UNDEFINED
                } else {
                    JsValue::from(StringArray::from(warnings))
                }
        }
        .unwrap();
        Ok(object.unchecked_into())
    }
}
