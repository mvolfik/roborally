use wasm_bindgen::{intern, prelude::wasm_bindgen};

use crate::create_array_type;

/// Transformation matrix
///
/// (0, 2)
/// (1, 3)
///
/// See <https://developer.mozilla.org/en-US/docs/Web/CSS/transform-function/matrix()>
#[derive(Clone, Copy, Default)]
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

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Direction {
    Up,
    Right,
    Down,
    Left,
}

impl Direction {
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
pub(crate) enum BeltEnd {
    /// ''
    Straight,
    /// 'l'
    TurnLeft,
    /// 'r'
    TurnRight,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum TileType {
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

impl Default for TileType {
    fn default() -> Self {
        Self::Void
    }
}

#[allow(clippy::struct_excessive_bools)]
#[derive(Clone, Copy, Default)]
pub(crate) struct WallsDescription {
    pub(crate) up: bool,
    pub(crate) right: bool,
    pub(crate) down: bool,
    pub(crate) left: bool,
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

#[derive(Clone, Copy, Default)]
pub(crate) struct Tile {
    pub(crate) typ: TileType,
    pub(crate) walls: WallsDescription,
}

impl Tile {
    #[must_use]
    pub(crate) fn get_assets(&self) -> Vec<Asset> {
        use BeltEnd::*;
        use TileType::*;

        let mut assets = match self.typ {
            Void => vec![Asset {
                uri: intern("void.png").to_string(),
                transform: Transform::default(),
            }],
            Floor => vec![Asset {
                uri: intern("floor.png").to_string(),
                transform: Transform::default(),
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
                        ..Transform::default()
                    },
                }]
            }
            Rotation(is_clockwise) => vec![Asset {
                uri: intern("rotate.png").to_string(),
                transform: Transform {
                    flip_x: !is_clockwise,
                    ..Transform::default()
                },
            }],
            PushPanel(dir, active_rounds) => {
                let mut assets = vec![Asset {
                    uri: intern("push-panel.png").to_string(),
                    transform: Transform {
                        rotate: dir.get_rotation(),
                        ..Transform::default()
                    },
                }];
                for (i, is_active) in active_rounds.iter().enumerate() {
                    #[allow(clippy::cast_precision_loss)]
                    assets.push(Asset {
                        uri: format!(
                            "push-panel-indicator-{}.png",
                            if *is_active { "active" } else { "inactive" }
                        ),
                        transform: Transform {
                            translate: Some(((7 + i * 10) as f64, 6.0)),
                            rotate: dir.get_rotation(),
                            ..Transform::default()
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
                        rotate: dir.get_rotation(),
                        ..Transform::default()
                    },
                });
            }
        }
        assets
    }
}

pub(crate) struct TileGrid(pub Vec<Tile>);

impl TileGrid {
    pub(crate) fn get_tile(
        &self,
        width: usize,
        height: usize,
        x: usize,
        y: usize,
    ) -> Option<&Tile> {
        if x >= width || y >= height {
            return None;
        }
        self.0.get((y * width + x) as usize)
    }
}
