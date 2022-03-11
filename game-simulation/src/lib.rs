mod utils;
use std::{iter::Peekable, str::Chars};

use wasm_bindgen::prelude::*;

#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[wasm_bindgen]
extern "C" {
    fn alert(s: &str);
}

#[wasm_bindgen]
pub fn greet() {
    alert("Hello, game-simulation!");
}

#[derive(Clone, Copy)]
pub enum Direction {
    Up,
    Right,
    Down,
    Left,
}
impl Direction {
    fn parse(it: &mut Peekable<Chars>) -> Result<Self, String> {
        use Direction::*;
        Ok(match it.next() {
            Some('u') => Up,
            Some('r') => Right,
            Some('d') => Down,
            Some('l') => Left,
            _ => return Err("Invalid direction specification".to_string()),
        })
    }

    fn as_transform(&self) -> Option<String> {
        use Direction::*;
        Some(format!(
            "rotate({}deg)",
            match self {
                Up => return None,
                Right => 90,
                Down => 180,
                Left => 270,
            }
        ))
    }
}

#[derive(Clone, Copy)]
pub enum BeltEnd {
    /// ''
    Straight,
    /// 'l'
    TurnLeft,
    /// 'r'
    TurnRight,
}

impl BeltEnd {
    fn parse(it: &mut Peekable<Chars>) -> Self {
        match it.peek() {
            Some('l') => {
                it.next();
                Self::TurnLeft
            }
            Some('r') => {
                it.next();
                Self::TurnRight
            }
            _ => Self::Straight,
        }
    }
}

#[derive(Clone, Copy)]
pub enum TileType {
    /// `V`
    Void,
    /// `F`
    Floor,
    /// `B(f|s){dir}[end]`
    /// bool = is_fast
    Belt(bool, Direction, BeltEnd),
    /// `P{dir}[1][2][3][4][5]`
    PushPanel(Direction, bool, bool, bool, bool, bool),
    /// `R(cw|ccw)`
    /// bool = is_clockwise
    Rotation(bool),
    /// `L{dir}(1-9)`
    Lasers(Direction, u8),
}

impl TileType {
    fn parse(it: &mut Peekable<Chars>) -> Result<Self, String> {
        use TileType::*;
        Ok(match it.next() {
            Some('V') => Void,
            Some('F') => Floor,
            Some('B') => match it.next() {
                None => return Err("Missing belt type".to_string()),
                Some(c @ ('f' | 's')) => Belt(c == 'f', Direction::parse(it)?, BeltEnd::parse(it)),
                Some(_) => return Err("Unknown belt type".to_string()),
            },
            Some('P') => {
                let mut last_char = '0';
                let mut digits = Vec::new();
                while let Some(d) = it.next_if(|c| {
                    let prev_char = last_char;
                    last_char = *c;
                    c > &prev_char && c <= &'6'
                }) {
                    digits.push(d);
                }
                PushPanel(
                    Direction::parse(it)?,
                    digits.contains(&'1'),
                    digits.contains(&'2'),
                    digits.contains(&'3'),
                    digits.contains(&'4'),
                    digits.contains(&'5'),
                )
            }
            Some('R') => match (it.next(), it.next(), it.peek()) {
                (Some('c'), Some('w'), _) => Rotation(true),
                (Some('c'), Some('c'), Some('w')) => {
                    it.next();
                    Rotation(false)
                }
                _ => return Err("Invalid rotation specification".to_string()),
            },
            Some('L') => Lasers(
                Direction::parse(it)?,
                it.next()
                    .map(|c| {
                        if c == '0' {
                            // disallow 0 lasers
                            None
                        } else {
                            c.to_digit(10)
                        }
                    })
                    .flatten()
                    .ok_or_else(|| "Invalid lasers specification".to_string())?
                    as u8,
            ),
            _ => return Err("Invalid tile specification".to_string()),
        })
    }
}

#[wasm_bindgen]
#[derive(Clone, Copy)]
pub struct WallsDescription {
    up: bool,
    right: bool,
    down: bool,
    left: bool,
}

#[wasm_bindgen]
#[derive(Clone)]
pub struct Asset {
    uri: String,
    transform: Option<String>,
}

#[wasm_bindgen]
impl Asset {
    pub fn uri(&self) -> String {
        self.uri.to_owned()
    }
    pub fn transform(&self) -> Option<String> {
        self.transform.to_owned()
    }
}

#[wasm_bindgen]
#[derive(Clone, Copy)]
pub struct Tile {
    typ: TileType,
    walls: WallsDescription,
}

#[wasm_bindgen]
impl Tile {
    pub fn walls(&self) -> WallsDescription {
        self.walls
    }
    pub fn get_assets(&self) -> Asset {
        use TileType::*;
        let mut assets = match self.typ {
            Void => vec![Asset {
                uri: "void.png".to_string(),
                transform: None,
            }],
            Floor => vec![Asset {
                uri: "floor.png".to_string(),
                transform: None,
            }],
            Belt(is_fast, dir, end) => {
                vec![Asset {
                    uri: if is_fast {
                        "fast-belt.png"
                    } else {
                        "slow-belt.png"
                    }
                    .to_string(),
                    transform: dir.as_transform(),
                }];
                todo!()
            }
            Rotation(true) => vec![Asset {
                uri: "rotate.png".to_string(),
                transform: None,
            }],
            Rotation(false) => vec![Asset {
                uri: "rotate.png".to_string(),
                transform: Some("scaleX(-1)".to_string()),
            }],
            _ => todo!(),
        };
        todo!()
    }
}

impl Tile {
    pub fn typ(&self) -> TileType {
        self.typ
    }
    fn parse(s: &str) -> Result<Self, String> {
        let it = &mut s.chars().peekable();
        let typ = TileType::parse(it)?;
        let mut walls = WallsDescription {
            up: false,
            right: false,
            down: false,
            left: false,
        };
        match it.next() {
            None => {}
            Some(':') => loop {
                let r = match it.next() {
                    None => break,
                    Some('u') => &mut walls.up,
                    Some('r') => &mut walls.right,
                    Some('d') => &mut walls.down,
                    Some('l') => &mut walls.left,
                    _ => return Err("Invalid wall specification".to_string()),
                };
                *r = true;
            },
            _ => return Err("Extra characters found at end".to_string()),
        }
        Ok(Self { typ, walls })
    }
}

#[wasm_bindgen]
pub struct GameMap {
    /// With coordinates starting at top left, index = x * width + y
    tiles: Vec<Tile>,
    width: usize,
    height: usize,
}

#[wasm_bindgen]
impl GameMap {
    pub fn get_tile(&self, x: usize, y: usize) -> Option<Tile> {
        if x >= self.width || y >= self.height {
            return None;
        }
        self.tiles.get(x * self.width + y).copied()
    }
    pub fn width(&self) -> usize {
        self.width
    }
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
                width = Some(this_line_width)
            }
        }
        Ok(GameMap {
            tiles,
            width: width.ok_or_else(|| "No tiles found".to_string())?,
            height,
        })
    }
}
