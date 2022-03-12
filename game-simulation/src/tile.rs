use std::{iter::Peekable, str::Chars};

use wasm_bindgen::prelude::wasm_bindgen;

/// Transformation matrix
///
/// (0, 2)
/// (1, 3)
///
/// See https://developer.mozilla.org/en-US/docs/Web/CSS/transform-function/matrix()
#[derive(Clone, Copy)]
struct Transform(f64, f64, f64, f64, bool);

impl Transform {
    fn combine_with(&self, other: &Self) -> Self {
        Self(
            self.0 * other.0 + self.2 * other.1,
            self.1 * other.0 + self.3 * other.1,
            self.0 * other.2 + self.2 * other.3,
            self.1 * other.2 + self.3 * other.3,
            self.4 ^ other.4,
        )
    }
    fn new() -> Self {
        Self(1.0, 0.0, 0.0, 1.0, false)
    }
    fn flip_x(&self) -> Self {
        self.combine_with(&Self(-1.0, 0.0, 0.0, 1.0, true))
    }
    fn rotate(&self, deg: u32) -> Self {
        let rad = (deg as f64).to_radians() * if self.4 { -1.0 } else { 1.0 };
        let (sin, cos) = rad.sin_cos();
        self.combine_with(&Self(cos, sin, -sin, cos, false))
    }
}

impl std::fmt::Display for Transform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "matrix({}, {}, {}, {}, 0, 0)",
            self.0, self.1, self.2, self.3
        )
    }
}

#[derive(Clone, Copy)]
enum Direction {
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

    /// By default, all directed tiles should point up
    fn get_degrees(&self) -> u32 {
        use Direction::*;
        match self {
            Up => 0,
            Right => 90,
            Down => 180,
            Left => 270,
        }
    }
}

#[derive(Clone, Copy)]
enum BeltEnd {
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
enum TileType {
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
                    .and_then(|c| {
                        if c == '0' {
                            // disallow 0 lasers
                            None
                        } else {
                            c.to_digit(10)
                        }
                    })
                    .ok_or_else(|| "Invalid lasers specification".to_string())?
                    as u8,
            ),
            _ => return Err("Invalid tile specification".to_string()),
        })
    }
}

#[derive(Clone, Copy)]
pub struct WallsDescription {
    up: bool,
    right: bool,
    down: bool,
    left: bool,
}

// #[wasm_bindgen(typescript_custom_section)]
// const ASSET_ARRAY: &'static str = "
// export type AssetArray = Asset[];
// ";

// #[wasm_bindgen]
// extern "C" {
//     #[wasm_bindgen(typescript_type = "AssetArray")]
//     pub type AssetArray;
// }

#[wasm_bindgen]
#[derive(Clone)]
pub struct Asset {
    uri: String,
    transform: Transform,
}

#[wasm_bindgen]
impl Asset {
    #[wasm_bindgen(getter)]
    pub fn uri(&self) -> String {
        self.uri.to_owned()
    }
    #[wasm_bindgen(getter)]
    pub fn transform_string(&self) -> String {
        self.transform.to_string()
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
    pub fn get_assets(&self) -> Asset {
        use BeltEnd::*;
        use TileType::*;
        let mut assets = match self.typ {
            Void => vec![Asset {
                uri: "void.png".to_string(),
                transform: Transform::new(),
            }],
            Floor => vec![Asset {
                uri: "floor.png".to_string(),
                transform: Transform::new(),
            }],
            Belt(is_fast, dir, end) => {
                let mut transform = Transform::new();
                if let TurnLeft = end {
                    transform = transform.flip_x();
                }
                transform = transform.rotate(dir.get_degrees());
                vec![Asset {
                    uri: format!(
                        "{}-belt-{}.png",
                        if is_fast { "fast" } else { "slow" },
                        if let Straight = end {
                            "straight"
                        } else {
                            "turn"
                        }
                    ),
                    transform,
                }]
            }
            Rotation(true) => vec![Asset {
                uri: "rotate.png".to_string(),
                transform: Transform::new(),
            }],
            Rotation(false) => vec![Asset {
                uri: "rotate.png".to_string(),
                transform: Transform::new().flip_x(),
            }],
            PushPanel(dir, a, b, c, d, e) => {
                let assets = vec![Asset {
                    uri: "push-panel.png".to_string(),
                    transform: Transform::new().rotate(dir.get_degrees()),
                }];
                for (_i, is_active) in [a, b, c, d, e].iter().enumerate() {
                    if *is_active {
                        todo!()
                    }
                }
                assets
            }
            Lasers(_, _) => todo!(),
        };
        for (is_wall, degrees) in [
            (self.walls.up, 0),
            (self.walls.right, 90),
            (self.walls.down, 180),
            (self.walls.left, 270),
        ] {
            if is_wall {
                assets.push(Asset {
                    uri: "wall.png".to_string(),
                    transform: Transform::new().rotate(degrees),
                });
            }
        }
        assets.get(0).unwrap().clone()
    }
}

impl Tile {
    pub(crate) fn parse(s: &str) -> Result<Self, String> {
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
                *match it.next() {
                    None => break,
                    Some('u') => &mut walls.up,
                    Some('r') => &mut walls.right,
                    Some('d') => &mut walls.down,
                    Some('l') => &mut walls.left,
                    _ => return Err("Invalid wall specification".to_string()),
                } = true;
            },
            _ => return Err("Extra characters found at end".to_string()),
        }
        Ok(Self { typ, walls })
    }
}
