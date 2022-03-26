use roborally_structs::{
    game_map::GameMap,
    position::Direction,
    tile::{Grid, Tile},
    tile_type::{BeltEnd, TileType},
};
use wasm_bindgen::{intern, prelude::wasm_bindgen};

use crate::create_array_type;

/// Transformation matrix
///
/// (0, 2)
/// (1, 3)
///
/// See <https://developer.mozilla.org/en-US/docs/Web/CSS/transform-function/matrix()>
#[non_exhaustive]
#[derive(Default, Clone, Copy)]
pub struct Transform {
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

#[wasm_bindgen]
#[derive(Clone)]
pub struct Asset {
    uri: String,
    transform: Transform,
}

create_array_type!( name: AssetArray, full_js_type: "Array<Asset>", rust_inner_type: Asset);

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
#[derive(Clone)]
pub struct TileAssets(Vec<Asset>);

#[wasm_bindgen]
impl TileAssets {
    pub fn to_jsarray(self) -> AssetArray {
        self.0.into()
    }
}

impl From<&Tile> for TileAssets {
    fn from(t: &Tile) -> Self {
        use BeltEnd::*;
        use TileType::*;

        let mut assets = match t.typ {
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
        };
        for (is_wall, dir) in [
            (t.walls.up, Direction::Up),
            (t.walls.right, Direction::Right),
            (t.walls.down, Direction::Down),
            (t.walls.left, Direction::Left),
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
        TileAssets(assets)
    }
}

#[wasm_bindgen]
#[derive(Clone)]
pub struct AssetMap(Grid<TileAssets>);

#[wasm_bindgen]
impl AssetMap {
    pub fn get(&self, x: usize, y: usize) -> Option<TileAssets> {
        self.0.get(x, y).cloned()
    }
    #[wasm_bindgen(getter)]
    pub fn width(&self) -> usize {
        self.0.size().x
    }
    #[wasm_bindgen(getter)]
    pub fn height(&self) -> usize {
        self.0.size().y
    }
}

#[allow(clippy::fallible_impl_from)]
impl From<GameMap> for AssetMap {
    fn from(m: GameMap) -> Self {
        let mut assets = m.tiles.map(|c| TileAssets::from(c));

        assets
            .get_mut(m.antenna.x, m.antenna.y)
            .unwrap()
            .0
            .push(Asset {
                uri: intern("antenna.png").to_string(),
                transform: Transform::default(),
            });

        assets
            .get_mut(m.reboot_token.0.x, m.reboot_token.0.y)
            .unwrap()
            .0
            .push(Asset {
                uri: intern("reboot-token.png").to_string(),
                transform: Transform {
                    rotate: m.reboot_token.1.get_rotation(),
                    ..Transform::default()
                },
            });

        for (i, checkpoint) in m.checkpoints.iter().enumerate() {
            assets
                .get_mut(checkpoint.x, checkpoint.y)
                .unwrap()
                .0
                .extend(
                    [
                        Asset {
                            uri: intern("checkpoint.png").to_string(),
                            transform: Transform::default(),
                        },
                        Asset {
                            uri: format!("number-{}.png", i + 1),
                            transform: Transform {
                                translate: Some((30.0, 30.0)),
                                ..Transform::default()
                            },
                        },
                    ]
                    .into_iter(),
                );
        }

        for (pos, dir) in m.spawn_points {
            assets.get_mut(pos.x, pos.y).unwrap().0.push(Asset {
                uri: intern("spawn-point.png").to_string(),
                transform: Transform {
                    rotate: dir.get_rotation(),
                    ..Transform::default()
                },
            });
        }

        Self(assets)
    }
}