use roborally_structs::{
    create_array_type,
    game_map::GameMap,
    position::{Direction, Position},
    tile::{DirectionBools, Grid, Tile},
    tile_type::TileType,
    transform::Effects,
};
use wasm_bindgen::{intern, prelude::wasm_bindgen};

#[wasm_bindgen]
#[derive(Clone)]
pub struct Asset {
    pub(crate) uri: String,
    pub(crate) effects: Effects,
}

create_array_type!( name: AssetArray, full_js_type: "Array<Asset>", rust_inner_type: Asset);

#[wasm_bindgen]
impl Asset {
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn uri(&self) -> String {
        intern(&self.uri).to_owned()
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
        AssetArray::from_iter(self.0.iter().cloned())
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
    pub fn get(&self, x: i16, y: i16) -> Option<TileAssets> {
        self.grid.get(Position { x, y }).cloned()
    }
    #[wasm_bindgen(getter)]
    pub fn width(&self) -> i16 {
        self.grid.size().x
    }
    #[wasm_bindgen(getter)]
    pub fn height(&self) -> i16 {
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
                            x: i as i16 % size.x,
                            y: i as i16 / size.x,
                        },
                        item,
                    )
                })
                .map(|(pos, tile)| {
                    use TileType::*;
                    let mut tile_assets = match tile.typ {
                        Void => vec![Asset {
                            uri: "floor.jpg".to_owned(),
                            effects: Effects {
                                scale: 0.25,
                                only_show_sides: Some(DirectionBools {
                                    up: m
                                        .tiles
                                        .get(pos.moved_in_direction(Direction::Up))
                                        .map_or(false, |t2| t2.typ != TileType::Void),
                                    right: m
                                        .tiles
                                        .get(pos.moved_in_direction(Direction::Right))
                                        .map_or(false, |t2| t2.typ != TileType::Void),
                                    down: m
                                        .tiles
                                        .get(pos.moved_in_direction(Direction::Down))
                                        .map_or(false, |t2| t2.typ != TileType::Void),
                                    left: m
                                        .tiles
                                        .get(pos.moved_in_direction(Direction::Left))
                                        .map_or(false, |t2| t2.typ != TileType::Void),
                                }),
                                ..Effects::random_rotate_flip()
                            },
                        }],
                        Floor => vec![Asset {
                            uri: "floor.jpg".to_owned(),
                            effects: Effects {
                                scale: 0.25,
                                ..Effects::random_rotate_flip()
                            },
                        }],
                        Belt(is_fast, dir) => {
                            let mut mask: u8 = 0b000;
                            for (i, possibly_incoming_belt_direction) in
                                [dir.rotated(), dir.rotated().rotated(), dir.rotated_ccw()]
                                    .into_iter()
                                    .enumerate()
                            {
                                if let Some(Tile {
                                        typ: Belt(is_fast2, dir2),
                                        ..
                                    }) = m.tiles.get(pos.moved_in_direction(possibly_incoming_belt_direction))
                                    && *is_fast2 == is_fast
                                    && *dir2 == possibly_incoming_belt_direction.rotated().rotated()
                                {
                                    mask |= 1 << i;
                                }
                            }
                            let flip_x = if mask & 0b101 == 0b100 {
                                mask ^= 0b101;
                                true
                            } else {
                                false
                            };

                            vec![Asset {
                                uri: format!(
                                    "{}-belt-{}.jpg",
                                    if is_fast { "fast" } else { "slow" },
                                    mask
                                ),
                                effects: Effects {
                                    flip_x,
                                    rotate: dir.to_continuous(),
                                    scale: 0.125,
                                    ..Effects::default()
                                },
                            }]
                        }
                        Rotation(is_clockwise) => vec![
                            Asset {
                                uri: "floor.jpg".to_owned(),
                                effects: Effects {
                                    scale: 0.25,
                                    ..Effects::random_rotate_flip()
                                },
                            },
                            Asset {
                                uri: "rotate.png".to_owned(),
                                effects: Effects {
                                    scale: 0.25,
                                    flip_x: !is_clockwise,
                                    ..Effects::default()
                                },
                            }
                        ],
                        PushPanel(dir, active_rounds) => {
                            let mut assets = vec![Asset {
                                uri: "push-panel.png".to_owned(),
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
                                        translate: Some(((2 + i * 12) as f64, 35.0)),
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
                                uri: "wall.png".to_owned(),
                                effects: Effects {
                                    rotate: dir.to_continuous(),
                                    scale: 0.25,
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
            uri: "antenna.png".to_owned(),
            effects: Effects {
                scale: 0.25,
                ..Effects::default()
            },
        });

        assets.get_mut(m.reboot_token.0).unwrap().0.push(Asset {
            uri: "reboot-token.png".to_owned(),
            effects: Effects {
                rotate: m.reboot_token.1.to_continuous(),
                ..Effects::default()
            },
        });

        for (i, checkpoint) in m.checkpoints.iter().enumerate() {
            assets.get_mut(*checkpoint).unwrap().0.extend(
                [
                    Asset {
                        uri: "checkpoint.png".to_owned(),
                        effects: Effects {
                            scale: 0.25,
                            ..Effects::default()
                        },
                    },
                    Asset {
                        uri: format!("number-{}.png", i + 1),
                        effects: Effects {
                            translate: Some((33.0, 35.0)),
                            ..Effects::default()
                        },
                    },
                ]
                .into_iter(),
            );
        }

        for (pos, dir) in m.lasers {
            assets.get_mut(pos).unwrap().0.push(Asset {
                uri: "laser.png".to_owned(),
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
