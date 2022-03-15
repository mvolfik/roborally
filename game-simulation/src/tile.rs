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
pub(crate) struct Transform {
    pub(crate) rotate: Option<f64>,
    pub(crate) flip_x: bool,
    pub(crate) translate: Option<(f64, f64)>,
}

impl std::fmt::Display for Transform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.flip_x {
            write!(f, "scaleX(-1)")?;
        }
        if let Some(deg) = self.rotate {
            write!(f, "rotate({}deg)", if self.flip_x { -deg } else { deg })?;
        }
        if let Some((x, y)) = self.translate {
            write!(
                f,
                "translate({}px,{}px)",
                if self.flip_x { -x } else { x },
                y
            )?;
        }
        Ok(())
    }
}

#[derive(Clone, Copy)]
pub(crate) enum Direction {
    Up,
    Right,
    Down,
    Left,
}

impl Direction {
    pub(crate) fn parse(c: char) -> Result<Self, String> {
        use Direction::*;
        Ok(match c {
            'u' => Up,
            'r' => Right,
            'd' => Down,
            'l' => Left,
            _ => return Err("Invalid direction specification".to_string()),
        })
    }

    /// By default, all directed tiles should point up
    #[must_use]
    pub(crate) const fn get_rotation(self) -> Option<f64> {
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
    PushPanel(Direction, [bool; 5]),
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
                Some(c @ ('f' | 's')) => Belt(
                    c == 'f',
                    Direction::parse(it.next().ok_or_else(|| "Need belt direction".to_string())?)?,
                    BeltEnd::parse(it),
                ),
                Some(_) => return Err("Unknown belt type".to_string()),
            },
            Some('P') => {
                let dir = Direction::parse(
                    it.next()
                        .ok_or_else(|| "Need push panel direction".to_string())?,
                )?;
                let mut active_rounds = [false; 5];
                let mut last_digit = 0;
                loop {
                    if let Some(Some(d)) = it.peek().map(|c| c.to_digit(10)) {
                        if d > last_digit && d <= 5 {
                            *active_rounds.get_mut(d as usize - 1).unwrap() = true;
                            last_digit = d;
                            it.next();
                            continue;
                        }
                    }
                    break;
                }
                PushPanel(dir, active_rounds)
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
                Direction::parse(
                    it.next()
                        .ok_or_else(|| "Need laser direction".to_string())?,
                )?,
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
struct WallsDescription {
    up: bool,
    right: bool,
    down: bool,
    left: bool,
}

#[wasm_bindgen]
#[derive(Clone)]
pub struct Asset {
    pub(crate) uri: String,
    pub(crate) transform: Transform,
}

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

create_array_type!( name: AssetArray, full_js_type: "Array<Asset>", rust_inner_type: Asset );

#[derive(Clone, Copy)]
pub(crate) struct Tile {
    typ: TileType,
    walls: WallsDescription,
}

impl Tile {
    #[must_use]
    pub(crate) fn get_assets(&self) -> Vec<Asset> {
        use BeltEnd::*;
        use TileType::*;

        let mut assets = match self.typ {
            Void => vec![Asset {
                uri: intern("void.png").to_string(),
                transform: Transform {
                    flip_x: false,
                    rotate: None,
                    translate: None,
                },
            }],
            Floor => vec![Asset {
                uri: intern("floor.png").to_string(),
                transform: Transform {
                    flip_x: false,
                    rotate: None,
                    translate: None,
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
                        rotate: dir.get_rotation(),
                        translate: None,
                    },
                }]
            }
            Rotation(is_clockwise) => vec![Asset {
                uri: intern("rotate.png").to_string(),
                transform: Transform {
                    flip_x: !is_clockwise,
                    rotate: None,
                    translate: None,
                },
            }],
            PushPanel(dir, active_rounds) => {
                let mut assets = vec![Asset {
                    uri: intern("push-panel.png").to_string(),
                    transform: Transform {
                        flip_x: false,
                        rotate: dir.get_rotation(),
                        translate: None,
                    },
                }];
                for (i, is_active) in active_rounds.iter().enumerate() {
                    assets.push(Asset {
                        uri: format!(
                            "push-panel-indicator-{}.png",
                            if *is_active { "active" } else { "inactive" }
                        ),
                        transform: Transform {
                            flip_x: false,
                            translate: Some(((7 + i * 10) as f64, 6.0)),
                            rotate: dir.get_rotation(),
                        },
                    });
                }
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
                        rotate: dir.get_rotation(),
                        translate: None,
                    },
                });
            }
        }
        assets
    }
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
