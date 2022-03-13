use std::{convert::Into, iter::Peekable, str::Chars};

use wasm_bindgen::{intern, prelude::wasm_bindgen};

use crate::create_array_type;

/// Transformation matrix
///
/// (0, 2)
/// (1, 3)
///
/// See <https://developer.mozilla.org/en-US/docs/Web/CSS/transform-function/matrix()>
#[derive(Clone, Copy)]
struct Transform {
    rotation: Option<f64>,
    flip_x: bool,
}

impl std::fmt::Display for Transform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.flip_x {
            write!(f, "scaleX(-1)")?;
        }
        if let Some(deg) = self.rotation {
            write!(f, "rotate({}deg)", if self.flip_x { -deg } else { deg })?;
        }
        Ok(())
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
    #[must_use]
    const fn get_rotation(self) -> Option<f64> {
        use Direction::*;
        match self {
            Up => None,
            Right => Some(90.0),
            Down => Some(180.0),
            Left => Some(-90.0),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum BeltEnd {
    /// ''
    Straight,
    /// 'l'
    TurnLeft,
    /// 'r'
    TurnRight,
}

impl BeltEnd {
    #[must_use]
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
                let dir = Direction::parse(it)?;
                let mut last_char = '0';
                let mut digits = Vec::new();
                while let Some(d) = it.next_if(|c| {
                    let prev_char = last_char;
                    last_char = *c;
                    c > &prev_char && c <= &'5'
                }) {
                    digits.push(d);
                }
                PushPanel(
                    dir,
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

            #[allow(clippy::cast_possible_truncation)]
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

#[allow(clippy::struct_excessive_bools)]
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
    transform: Transform,
}

create_array_type!( name: AssetArray, full_js_type: "Array<Asset>", rust_inner_type: Asset );

#[wasm_bindgen]
impl Asset {
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn uri(&self) -> String {
        self.uri.clone()
    }
    #[wasm_bindgen(getter)]
    #[must_use]
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
    #[must_use]
    pub fn get_assets(&self) -> AssetArray {
        use BeltEnd::*;
        use TileType::*;

        let mut assets = match self.typ {
            Void => vec![Asset {
                uri: intern("void.png").to_string(),
                transform: Transform {
                    flip_x: false,
                    rotation: None,
                },
            }],
            Floor => vec![Asset {
                uri: intern("floor.png").to_string(),
                transform: Transform {
                    flip_x: false,
                    rotation: None,
                },
            }],
            Belt(is_fast, dir, end) => {
                vec![Asset {
                    uri: format!(
                        "{}-belt-{}.png",
                        if is_fast { "fast" } else { "slow" },
                        if end == Straight { "straight" } else { "turn" }
                    ),
                    transform: Transform {
                        flip_x: end == BeltEnd::TurnLeft,
                        rotation: dir.get_rotation(),
                    },
                }]
            }
            Rotation(is_clockwise) => vec![Asset {
                uri: intern("rotate.png").to_string(),
                transform: Transform {
                    flip_x: !is_clockwise,
                    rotation: None,
                },
            }],
            PushPanel(dir, a, b, c, d, e) => {
                let assets = vec![Asset {
                    uri: intern("push-panel.png").to_string(),
                    transform: Transform {
                        flip_x: false,
                        rotation: dir.get_rotation(),
                    },
                }];
                let _active_rounds: Vec<_> = [a, b, c, d, e]
                    .iter()
                    .enumerate()
                    .filter_map(|(i, is_active)| if *is_active { Some(i) } else { None })
                    .collect();
                // assets.extend(active_rounds.iter().map(|i| Asset {
                //     uri: format!("number-{}.png", i + 1),
                //     transform: todo!(),
                // }));
                assets
            }
            Lasers(_, _) => todo!(),
        };
        for (is_wall, dir) in [
            (self.walls.up, Direction::Up),
            (self.walls.right, Direction::Right),
            (self.walls.down, Direction::Down),
            (self.walls.left, Direction::Left),
        ] {
            if is_wall {
                assets.push(Asset {
                    uri: intern("wall.png").to_string(),
                    transform: Transform {
                        flip_x: false,
                        rotation: dir.get_rotation(),
                    },
                });
            }
        }
        assets.into()
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
