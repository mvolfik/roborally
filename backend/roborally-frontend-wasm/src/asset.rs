use roborally_structs::{
    game_map::GameMap,
    position::{Direction, Position},
    tile::{DirectionBools, Grid},
    tile_type::{BeltEnd, TileType},
    transform::Effects,
};
use wasm_bindgen::{intern, prelude::wasm_bindgen};

use crate::create_array_type;

#[wasm_bindgen]
#[derive(Clone)]
pub struct Asset {
    uri: String,
    effects: Effects,
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
    pub fn style(&self) -> String {
        self.effects.to_string()
    }
}

#[wasm_bindgen]
#[derive(Clone)]
pub struct TileAssets(Vec<Asset>);

#[wasm_bindgen]
impl TileAssets {
    pub fn into_jsarray(self) -> AssetArray {
        self.0.into()
    }
}

#[wasm_bindgen]
#[derive(Clone)]
pub struct AssetMap {
    grid: Grid<TileAssets>,
    #[wasm_bindgen(readonly)]
    pub checkpoints: usize,
}

#[wasm_bindgen]
impl AssetMap {
    pub fn get(&self, x: usize, y: usize) -> Option<TileAssets> {
        self.grid.get(Position { x, y }).cloned()
    }
    #[wasm_bindgen(getter)]
    pub fn width(&self) -> usize {
        self.grid.size().x
    }
    #[wasm_bindgen(getter)]
    pub fn height(&self) -> usize {
        self.grid.size().y
    }
}

#[allow(clippy::fallible_impl_from)]
impl From<GameMap> for AssetMap {
    fn from(m: GameMap) -> Self {
        let size = m.tiles.size();
        let mut assets: Grid<TileAssets> = Grid::new(
            m.tiles
                .vec()
                .iter()
                .enumerate()
                .map(|(i, item)| {
                    (
                        Position {
                            x: i % size.x,
                            y: i / size.x,
                        },
                        item,
                    )
                })
                .map(|(pos, tile)| {
                    use BeltEnd::*;
                    use TileType::*;
                    let mut tile_assets = match tile.typ {
                        Void => vec![Asset {
                            uri: intern("floor.jpg").to_owned(),
                            effects: Effects {
                                scale: 0.25,
                                only_show_sides: Some(DirectionBools {
                                    up: m
                                        .tiles
                                        .get(Direction::Up.apply_to(&pos))
                                        .map_or(false, |t2| t2.typ != TileType::Void),
                                    right: m
                                        .tiles
                                        .get(Direction::Right.apply_to(&pos))
                                        .map_or(false, |t2| t2.typ != TileType::Void),
                                    down: m
                                        .tiles
                                        .get(Direction::Down.apply_to(&pos))
                                        .map_or(false, |t2| t2.typ != TileType::Void),
                                    left: m
                                        .tiles
                                        .get(Direction::Left.apply_to(&pos))
                                        .map_or(false, |t2| t2.typ != TileType::Void),
                                }),
                                ..Effects::random_rotate_flip()
                            },
                        }],
                        Floor => vec![Asset {
                            uri: intern("floor.jpg").to_owned(),
                            effects: Effects {
                                scale: 0.25,
                                ..Effects::random_rotate_flip()
                            },
                        }],
                        Belt(is_fast, dir, end) => {
                            vec![Asset {
                                uri: format!(
                                    "{}-belt-{}.png",
                                    if is_fast { "fast" } else { "slow" },
                                    if end == Straight { "straight" } else { "turn" }
                                ),
                                effects: Effects {
                                    flip_x: end == BeltEnd::TurnLeft,
                                    rotate: dir.to_continuous(),
                                    ..Effects::default()
                                },
                            }]
                        }
                        Rotation(is_clockwise) => vec![Asset {
                            uri: intern("rotate.png").to_owned(),
                            effects: Effects {
                                flip_x: !is_clockwise,
                                ..Effects::default()
                            },
                        }],
                        PushPanel(dir, active_rounds) => {
                            let mut assets = vec![Asset {
                                uri: intern("push-panel.png").to_owned(),
                                effects: Effects {
                                    rotate: dir.to_continuous(),
                                    ..Effects::default()
                                },
                            }];
                            for (i, is_active) in active_rounds.iter().enumerate() {
                                #[allow(clippy::cast_precision_loss)]
                                assets.push(Asset {
                                    uri: format!(
                                        "push-panel-indicator-{}.png",
                                        if *is_active { "active" } else { "inactive" }
                                    ),
                                    effects: Effects {
                                        translate: Some(((2 + i * 12) as f64, 42.0)),
                                        rotate: dir.to_continuous(),
                                        ..Effects::default()
                                    },
                                });
                            }
                            assets
                        }
                    };

                    for (dir, is_wall) in tile.walls.to_items() {
                        if is_wall {
                            tile_assets.push(Asset {
                                uri: intern("wall.png").to_owned(),
                                effects: Effects {
                                    rotate: dir.to_continuous(),
                                    ..Effects::default()
                                },
                            });
                        }
                    }
                    TileAssets(tile_assets)
                })
                .collect(),
            size,
        )
        .unwrap();

        assets.get_mut(m.antenna).unwrap().0.push(Asset {
            uri: intern("antenna.png").to_owned(),
            effects: Effects::default(),
        });

        assets.get_mut(m.reboot_token.0).unwrap().0.push(Asset {
            uri: intern("reboot-token.png").to_owned(),
            effects: Effects {
                rotate: m.reboot_token.1.to_continuous(),
                ..Effects::default()
            },
        });

        for (i, checkpoint) in m.checkpoints.iter().enumerate() {
            assets.get_mut(*checkpoint).unwrap().0.extend(
                [
                    Asset {
                        uri: intern("checkpoint.png").to_owned(),
                        effects: Effects::default(),
                    },
                    Asset {
                        uri: format!("number-{}.png", i + 1),
                        effects: Effects {
                            translate: Some((30.0, 30.0)),
                            ..Effects::default()
                        },
                    },
                ]
                .into_iter(),
            );
        }

        for (pos, dir) in m.spawn_points {
            assets.get_mut(pos).unwrap().0.push(Asset {
                uri: intern("spawn-point.png").to_owned(),
                effects: Effects {
                    rotate: dir.to_continuous(),
                    ..Effects::default()
                },
            });
        }

        for (pos, dir) in m.lasers {
            assets.get_mut(pos).unwrap().0.push(Asset {
                uri: intern("laser.png").to_owned(),
                effects: Effects {
                    rotate: dir.to_continuous(),
                    ..Effects::default()
                },
            });
        }

        Self {
            grid: assets,
            checkpoints: m.checkpoints.len(),
        }
    }
}
