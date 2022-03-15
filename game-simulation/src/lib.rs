#![warn(clippy::nursery)]
#![allow(clippy::use_self)]
#![warn(clippy::pedantic)]
#![allow(clippy::unused_unit)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::enum_glob_use)]
#![allow(clippy::many_single_char_names)]
#![feature(pattern)]
pub mod store;
pub mod tile;
mod utils;

use std::{collections::HashMap, convert::Into, panic, str::FromStr};
use tile::{AssetArray, Direction};
use utils::StringArray;
use wasm_bindgen::{prelude::*, JsCast};

use crate::tile::{Asset, Tile, Transform};

///// INIT /////
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[wasm_bindgen(start)]
pub fn init_panic_hook() {
    panic::set_hook(Box::new(console_error_panic_hook::hook));
}
///// /INIT /////

#[derive(Clone, Copy, PartialEq, Eq)]
struct Position {
    x: u32,
    y: u32,
}

impl Position {
    // #[must_use]
    // fn new(x: u32, y: u32) -> Self {
    //     Self { x, y }
    // }
    fn parse(it: &str) -> Result<Self, String> {
        let (x_str, y_str) = checked_split_in_two(it, ',')
            .ok_or_else(|| format!("Value `{}` doesn't have format `x,y`", it))?;
        Ok(Self {
            x: u32::from_str(x_str).map_err(|_e| "Expected a number (x coordinate)".to_string())?,
            y: u32::from_str(y_str).map_err(|_e| "Expected a number (x coordinate)".to_string())?,
        })
    }

    /// Whether the rectangle [origin, self) contains other
    #[inline]
    #[allow(clippy::missing_const_for_fn)]
    fn contains(self, other: Self) -> bool {
        other.x < self.x && other.y < self.y
    }
}

#[wasm_bindgen]
pub struct GameMap {
    /// With coordinates starting at top left, index = x * width + y
    tiles: Vec<Tile>,
    size: Position,
    antenna: Position,
    reboot_token: (Position, Direction),
    checkpoints: Vec<Position>,
    spawn_points: Vec<(Position, Direction)>,
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
    fn get_tile(&self, x: u32, y: u32) -> Option<&Tile> {
        if x >= self.size.x || y >= self.size.y {
            return None;
        }
        self.tiles.get((y * self.size.x + x) as usize)
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
                transform: Transform {
                    flip_x: false,
                    rotate: None,
                    translate: None,
                },
            });
        }
        if self.reboot_token.0 == this_pos {
            assets.push(Asset {
                uri: "reboot-token.png".to_string(),
                transform: Transform {
                    flip_x: false,
                    rotate: self.reboot_token.1.get_rotation(),
                    translate: None,
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
                transform: Transform {
                    flip_x: false,
                    rotate: None,
                    translate: None,
                },
            });
            assets.push(Asset {
                uri: format!("number-{}.png", i + 1),
                transform: Transform {
                    flip_x: false,
                    rotate: None,
                    translate: Some((30.0, 30.0)),
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
                    flip_x: false,
                    rotate: dir.get_rotation(),
                    translate: None,
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
    pub fn parse(s: &str) -> Result<GameMapParseOkResult, String> {
        let mut warnings = Vec::new();
        let mut lines = s.lines();
        let first_line = &mut lines
            .next()
            .ok_or_else(|| "No lines in input".to_string())?;

        let mut props = HashMap::new();
        for propdef in first_line.split(' ') {
            let (name, value) = checked_split_in_two(propdef, '=').ok_or_else(|| {
                format!(
                    "Prop definition `{}` doesn't follow format key=value",
                    propdef
                )
            })?;
            props.insert(name, value);
        }
        let size = Position::parse(
            props
                .remove("Size")
                .ok_or_else(|| "Must specify 'Size' (grid dimensions) in header".to_string())?,
        )
        .map_err(|e| format!("Error parsing value for Size: {}", e))?;
        if size.x == 0 || size.y == 0 {
            return Err("Map dimensions must be non-zero".to_string());
        }

        let antenna =
            Position::parse(props.remove("Antenna").ok_or_else(|| {
                "Must specify 'Antenna' (antenna position) in header".to_string()
            })?)
            .map_err(|e| format!("Error parsing value for Antenna: {}", e))?;
        if !size.contains(antenna) {
            return Err("Antenna must be within map bounds".to_string());
        }

        let reboot_token = {
            let (pos_spec, dir_spec) = checked_split_in_two(
                props.remove("Reboot").ok_or_else(|| {
                    "Must specify 'Reboot' (reboot token position) in header".to_string()
                })?,
                ':',
            )
            .ok_or_else(|| "Reboot token must be specified as `position:direction`".to_string())?;
            if dir_spec.len() != 1 {
                return Err("Only need 1 character for direction".to_string());
            }

            let pos = Position::parse(pos_spec)
                .map_err(|e| format!("Error parsing value for Reboot: {}", e))?;
            if !size.contains(pos) {
                return Err("Reboot token must be within map bounds".to_string());
            }
            let dir = Direction::parse(dir_spec.chars().next().unwrap())?;
            (pos, dir)
        };

        let checkpoints = props
            .remove("Checkpoints")
            .ok_or_else(|| "Must specify 'Checkpoints' in header".to_string())?
            .split(';')
            .enumerate()
            .map(|(i, pos_str)| {
                let pos = Position::parse(pos_str)
                    .map_err(|e| format!("Error parsing value for Checkpoints[{}]: {}", i, e))?;
                if !size.contains(pos) {
                    return Err("Checkpoint must be within map bounds".to_string());
                }
                Ok(pos)
            })
            .collect::<Result<_, _>>()?;

        let spawn_points = props
            .remove("Spawnpoints")
            .ok_or_else(|| "Must specify 'Spawnpoints' in header".to_string())?
            .split(';')
            .enumerate()
            .map(|(i, spec)| {
                let (pos_spec, dir_spec) = checked_split_in_two(spec, ':')
                    .ok_or_else(|| "Spawn must be specified as `position:direction`".to_string())?;
                let pos = Position::parse(pos_spec)
                    .map_err(|e| format!("Error parsing value for Spawnpoints[{}]: {}", i, e))?;
                if !size.contains(pos) {
                    return Err("Spawn point must be within map bounds".to_string());
                }
                if dir_spec.len() != 1 {
                    return Err("Only need 1 character for direction".to_string());
                }
                let dir = Direction::parse(dir_spec.chars().next().unwrap())?;
                Ok((pos, dir))
            })
            .collect::<Result<_, _>>()?;

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

            if x != size.x {
                return Err(format!(
                    "Line {} contains wrong amount of tiles (found {}, expected width={})",
                    y, x, size.x
                ));
            }
            y += 1;
        }
        if y != size.y {
            return Err(format!(
                "Wrong amount of tile lines (found {}, expected height={})",
                y, size.y
            ));
        }
        let map = Self {
            tiles,
            size,
            antenna,
            reboot_token,
            checkpoints,
            spawn_points,
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

fn checked_split_in_two<'a, T: std::str::pattern::Pattern<'a>>(
    s: &'a str,
    delimiter: T,
) -> Option<(&'a str, &'a str)> {
    let mut split = s.split(delimiter);
    if let (Some(a), Some(b), None) = (split.next(), split.next(), split.next()) {
        Some((a, b))
    } else {
        None
    }
}
