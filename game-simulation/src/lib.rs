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

use js_sys::{Array, Object, Reflect};
use std::{collections::HashMap, convert::Into, iter::Peekable, panic, str::Chars};
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
    fn parse(it: &mut Peekable<Chars>) -> Result<Self, String> {
        let x = parse_number(it).ok_or_else(|| "Expected a number (x coordinate)".to_string())?;
        if it.next() != Some(',') {
            return Err("Expected ',' after x coordinate".to_string());
        }
        let y = parse_number(it).ok_or_else(|| "Expected a number (y coordinate)".to_string())?;
        Ok(Self { x, y })
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
    /// Size={width},{height};Antenna={x},{y}
    ///
    /// Then follow {width} remaining lines
    pub fn parse(s: &str) -> Result<GameMapParseOkResult, String> {
        let mut warnings = Vec::new();
        let mut lines = s.lines();
        let first_line = &mut lines
            .next()
            .ok_or_else(|| "No lines in input".to_string())?
            .chars()
            .peekable();
        let mut parsed_props = HashMap::new();
        loop {
            let mut name = String::new();
            loop {
                match first_line.next() {
                    None => {
                        return Err("Unexpected EOL".to_string());
                    }
                    Some('=') => break,
                    Some(c) => {
                        if c.is_ascii_alphabetic() {
                            name.push(c);
                        } else {
                            return Err(format!("Unexpected character: {}", c));
                        }
                    }
                }
            }
            if name.is_empty() {
                return Err("Found zero-length name".to_string());
            }
            let pos = Position::parse(first_line)
                .map_err(|e| format!("Error parsing value for {}: {}", name, e))?;
            parsed_props.insert(name, pos);

            match first_line.next() {
                None => break,
                Some(';') => {}
                Some(_) => {
                    return Err("Expected ';' after value".to_string());
                }
            }
        }
        let Position {
            x: width,
            y: height,
        } = parsed_props
            .remove("Size")
            .ok_or_else(|| "Must specify 'Size' in header".to_string())?;

        for name in parsed_props.keys() {
            warnings.push(format!("Unused prop in header: {}", name))
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
        let object = Object::new();
        Reflect::set(&object, &"map".into(), &map.into()).unwrap();
        if warnings.len() > 0 {
            Reflect::set(
                &object,
                &"warnings".into(),
                &warnings
                    .into_iter()
                    .map::<JsValue, _>(Into::into)
                    .collect::<Array>(),
            )
            .unwrap();
        }
        Ok(object.unchecked_into())
    }
}

#[must_use]
fn parse_number(it: &mut Peekable<Chars>) -> Option<u32> {
    let mut out = it.next()?.to_digit(10)?;
    while let Some(Some(n)) = it.peek().map(|c| c.to_digit(10)) {
        it.next();
        out = out * 10 + n;
    }
    Some(out)
}
