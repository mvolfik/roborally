use std::{collections::HashMap, fmt::Debug, future::Future, mem, sync::Weak};

use roborally_structs::{
    animations::Animation,
    game_state::{
        animated_state::{AnimationItem, RunningStateView},
        phase::RegisterMovePhase,
        GameStatusInfo, GeneralState, ProgrammingState,
    },
    position::{ContinuousDirection, Direction, Position, Priority},
    tile::Tile,
    tile_type::TileType,
    transport::ServerMessage,
};

use crate::{game::Game, game_connection::SocketMessage::SendMessage, player::Player};

pub struct BoxedFuture(pub Box<dyn Future<Output = ()> + Send + Sync + Unpin + 'static>);

impl Debug for BoxedFuture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BoxedFuture").finish_non_exhaustive()
    }
}

pub struct GameState {
    /// No logic should be tied to the status, it's purely presentational
    pub status: GameStatusInfo,
    pub players: Vec<Player>,
    pub game: Weak<Game>,
    pub winner: Option<usize>,
    pub reboot_queue: Vec<usize>,
    /// It isn't great that this has to be here, but it would be too messy to pass this all over the place.
    /// Conversion into PlayerGameStateView needs to have access to this.
    pub running_state: (usize, RegisterMovePhase),
}

#[derive(Clone, Copy, Debug)]
pub struct MoveResult {
    pub moved: bool,
    pub reboot: bool,
}

impl GameState {
    pub fn send_general_state(&self) {
        let player_connections = self
            .players
            .iter()
            .map(|p| p.connected.upgrade())
            .collect::<Vec<_>>();
        let state = ServerMessage::GeneralState(GeneralState {
            player_names: player_connections
                .iter()
                .map(|conn_opt| conn_opt.as_ref().map(|conn| conn.player_name.clone()))
                .collect(),
            status: self.status.clone(),
        });
        for conn in player_connections.into_iter().flatten() {
            conn.sender.send(SendMessage(state.clone())).unwrap();
        }
    }

    pub fn send_programming_state_to_player(&self, player_i: usize) {
        let player = &self.players[player_i];
        let Some(conn) = player.connected.upgrade() else {
            return;
        };
        let state = ServerMessage::ProgrammingState(ProgrammingState {
            hand: player.hand.clone(),
            prepared_cards: player.prepared_cards.clone(),
            ready_players: self
                .players
                .iter()
                .map(|p| p.prepared_cards.is_some())
                .collect(),
            player_states: self
                .players
                .iter()
                .map(|p| p.public_state.clone())
                .collect(),
        });
        conn.sender.send(SendMessage(state)).unwrap();
    }

    pub fn send_programming_state_to_all(&self) {
        for i in 0..self.players.len() {
            self.send_programming_state_to_player(i);
        }
    }

    pub fn send_animation_item(&self, animations: &[Animation], include_state: bool) {
        for player in &self.players {
            let Some(conn) = player.connected.upgrade()
            else {
                continue;
            };
            let state = ServerMessage::AnimatedState(AnimationItem {
                animations: animations.to_vec(),
                state: include_state.then(|| RunningStateView {
                    register: self.running_state.0,
                    register_phase: self.running_state.1,
                    my_cards: player.prepared_cards.as_ref().unwrap().clone(),
                    players_revealed_cards: self
                        .players
                        .iter()
                        .map(|p| {
                            p.prepared_cards.as_ref().unwrap()[..=self.running_state.0].to_vec()
                        })
                        .collect(),
                    player_states: self
                        .players
                        .iter()
                        .map(|p| p.public_state.clone())
                        .collect(),
                }),
            });
            conn.sender.send(SendMessage(state)).unwrap();
        }
    }

    pub fn send_log(&self, log: &str) {
        for player in &self.players {
            let Some(conn) = player.connected.upgrade()
            else {
                continue;
            };
            let state = ServerMessage::GameLog(log.to_owned());
            conn.sender.send(SendMessage(state)).unwrap();
        }
    }

    /// Returns the index of the player at the given position, or None if there is no player there.
    pub fn player_at_position(&self, position: Position) -> Option<usize> {
        let mut players = self
            .players
            .iter()
            .enumerate()
            .filter_map(|(i, player)| (player.public_state.position == position).then_some(i));
        let result = players.next();
        if players.next().is_some() {
            let game = self.game.upgrade().unwrap();
            let message = if game
                .map
                .tiles
                .get(position)
                .is_some_and(|t| t.typ != TileType::Void)
            {
                format!(
                    "Internal state error: more than 1 player on tile {},{}",
                    position.x, position.y
                )
            } else {
                "Recommendation violation: called player_at_position on a void tile, and there was more than 1 player there".to_owned()
            };
            game.log.lock().unwrap().push_str(&message);
        }
        result
    }

    pub fn mov(
        &mut self,
        player_i: usize,
        direction: impl Into<Direction>,
    ) -> Result<MoveResult, String> {
        let map = &self.game.upgrade().unwrap().map;
        let player = self
            .players
            .get_mut(player_i)
            .ok_or_else(|| "There aren't that many players".to_owned())?;
        let origin_pos = player.public_state.position;

        let origin_tile = match map.tiles.get(origin_pos) {
            None
            | Some(Tile {
                typ: TileType::Void,
                ..
            }) => {
                return Ok(MoveResult {
                    moved: true,
                    reboot: false,
                })
            }
            Some(t) => t,
        };
        let direction = direction.into();
        if origin_tile.walls.get(direction) {
            return Ok(MoveResult {
                moved: false,
                reboot: false,
            });
        }
        let target_pos = origin_pos.moved_in_direction(direction);
        let Some(target_tile) = map.tiles.get(target_pos)
        else {
            // falling out of map
            player.public_state.position = target_pos;
            self.reboot_queue.push(player_i);
            return Ok(MoveResult { moved: true, reboot: true });
        };
        if target_tile.walls.get(direction.rotated().rotated()) {
            return Ok(MoveResult {
                moved: false,
                reboot: false,
            });
        }
        if target_tile.typ == TileType::Void {
            player.public_state.position = target_pos;
            self.reboot_queue.push(player_i);
            return Ok(MoveResult {
                moved: true,
                reboot: true,
            });
        }

        if let Some(player2_i) = self.player_at_position(target_pos) {
            if !self.mov(player2_i, direction)?.moved {
                return Ok(MoveResult {
                    moved: false,
                    reboot: false,
                });
            }
        }
        self.players[player_i].public_state.position = target_pos;
        Ok(MoveResult {
            moved: true,
            reboot: false,
        })
    }

    pub fn force_move_to(
        &mut self,
        player_i: usize,
        pos: Position,
        pushing_direction: Direction,
    ) -> Result<MoveResult, String> {
        let map = &self.game.upgrade().unwrap().map;
        let player = self
            .players
            .get_mut(player_i)
            .ok_or_else(|| "There aren't that many players".to_owned())?;
        if let None
        | Some(Tile {
            typ: TileType::Void,
            ..
        }) = map.tiles.get(pos)
        {
            player.public_state.position = pos;
            self.reboot_queue.push(player_i);
            return Ok(MoveResult {
                moved: true,
                reboot: true,
            });
        };

        if let Some(player2_i) = self.player_at_position(pos) && player2_i != player_i {
            assert!(
                self.force_move_to(
                    player2_i,
                    pos.moved_in_direction(pushing_direction),
                    pushing_direction,
                )
                .unwrap()
                .moved,
            );
        }
        self.players[player_i].public_state.position = pos;
        Ok(MoveResult {
            moved: true,
            reboot: false,
        })
    }

    /// Hide all players that should reboot, queue a state update, then move them one by one to the reboot token
    ///
    /// This should typically be called directly after a move, without any intermediate state updates
    pub fn execute_reboots(&mut self) {
        for player_i in &self.reboot_queue {
            self.players[*player_i].public_state.is_hidden = true;
        }
        self.send_animation_item(&[], true);

        let game = self.game.upgrade().unwrap();
        let reboot_token = game.map.reboot_token;
        for player_i in mem::take(&mut self.reboot_queue) {
            let player = &mut self.players[player_i];
            player.draw_spam();
            player.draw_spam();
            player.public_state.direction = player
                .public_state
                .direction
                .closest_in_given_basic_direction(reboot_token.1);
            player.public_state.is_rebooting = true;
            player.public_state.is_hidden = false;

            // Temporarily move player away, to prevent collisions with players pushed during reboot.
            // This isn't necessary as of now - if the robot is rebooting, it is in a hole, and if a
            // robot pushed away by a reboot ends up in a hole, we're entering a panic anyways.
            // However, this would become needed if the Worm damage card (or other means of reboot)
            // are later introduced
            player.public_state.position.x = i16::MAX;
            self.force_move_to(player_i, reboot_token.0, reboot_token.1)
                .unwrap();

            self.send_animation_item(&[], true);
        }
        assert!(self.reboot_queue.is_empty());
    }

    pub fn player_indices_by_priority(&self) -> Vec<usize> {
        let mut result: Vec<usize> = (0..self.players.len()).collect();
        let antenna = self.game.upgrade().unwrap().map.antenna;
        result.sort_by_key(|i| Priority::new(self.players[*i].public_state.position, antenna));
        result
    }

    pub fn execute_belts(&mut self, fast: bool) {
        //   Mapping Position -> Indexes of all players on that tile (can't store players directly due to borrow rules)
        let mut moved_positions: HashMap<Position, Vec<(usize, ContinuousDirection)>> =
            HashMap::new();
        let game = self.game.upgrade().unwrap();

        for (player_i, player) in self.players.iter_mut().enumerate() {
            let mut player_pos = player.public_state.position;
            let mut player_dir = player.public_state.direction;

            if let Some(Tile {
                typ: TileType::Belt(is_fast, belt_dir),
                walls,
            }) = game.map.tiles.get(player_pos)
            && *is_fast == fast
            && !walls.get(*belt_dir)
            // is on belt and can leave current tile
            {
                let new_pos = player_pos.moved_in_direction(*belt_dir);
                let new_tile = game.map.tiles.get(new_pos);
                if !new_tile.is_some_and(|t| t.walls.get(belt_dir.rotated().rotated())) {
                    // actually move, now just need to potentially rotate
                    player_pos = new_pos;
                    player_dir = if let Some(Tile {
                        typ: TileType::Belt(is_fast2, dir2),
                        ..
                    }) = new_tile
                    && *is_fast2 == fast
                    {
                        if *dir2 == belt_dir.rotated() {
                            player_dir.rotated()
                        } else if *dir2 == belt_dir.rotated_ccw() {
                            player_dir.rotated_ccw()
                        } else {
                            player_dir
                        }
                    } else {
                        player_dir
                    };
                }
            }
            moved_positions
                .entry(player_pos)
                .or_default()
                .push((player_i, player_dir));
        }

        loop {
            let mut made_changes = false;
            for (position, players) in mem::take(&mut moved_positions) {
                // conflicts out of bounds or on void tiles don't matter
                if players.len() > 1
                    && game
                        .map
                        .tiles
                        .get(position)
                        .is_some_and(|t| t.typ != TileType::Void)
                {
                    made_changes = true;
                    // move all players on the conflicted tile to the original position
                    // -> those that were attempted to be moved by a belt are reset, the rest stay
                    for (player_i, _) in players {
                        let player_state = &self.players[player_i].public_state;
                        moved_positions
                            .entry(player_state.position)
                            .or_default()
                            .push((player_i, player_state.direction));
                    }
                } else {
                    // no conflict -> simply copy back
                    moved_positions.entry(position).or_default().extend(players);
                }
            }
            if !made_changes {
                break;
            }
        }
        let mut any_moved = false;
        let mut to_reboot = Vec::new();
        for (position, players) in moved_positions {
            let should_reboot = !game
                .map
                .tiles
                .get(position)
                .is_some_and(|t| t.typ != TileType::Void);
            for (player_i, direction) in &players {
                let player_state = &mut self.players[*player_i].public_state;
                if player_state.position == position && player_state.direction == *direction {
                    continue;
                }
                any_moved = true;
                if should_reboot {
                    // priority somehow needs to be determined - use position before the move
                    to_reboot.push((*player_i, player_state.position));
                }
                player_state.position = position;
                player_state.direction = *direction;
            }
        }
        to_reboot.sort_by_key(|(_, pos)| Priority::new(*pos, game.map.antenna));
        self.reboot_queue
            .extend(to_reboot.into_iter().map(|(player_i, _)| player_i));
        if any_moved {
            self.execute_reboots();
        }
    }

    pub fn execute_push_panels(&mut self, register_i: usize) {
        let map = &self.game.upgrade().unwrap().map;
        for player_i in self.player_indices_by_priority() {
            let pos = self.players[player_i].public_state.position;
            if let TileType::PushPanel(dir, divisor, remainder) = map.tiles.get(pos).unwrap().typ {
                if (register_i + 1) % divisor == remainder {
                    self.mov(player_i, dir).unwrap();
                    self.execute_reboots();
                }
            }
        }
    }

    pub fn execute_rotators(&mut self) {
        let map = &self.game.upgrade().unwrap().map;
        let mut any_rotated = false;
        for player in &mut self.players {
            if let TileType::Rotation(is_cw) =
                map.tiles.get(player.public_state.position).unwrap().typ
            {
                let dir = &mut player.public_state.direction;
                *dir = if is_cw {
                    dir.rotated()
                } else {
                    dir.rotated_ccw()
                };
                any_rotated = true;
            }
        }
        if any_rotated {
            self.send_animation_item(&[], true);
        }
    }

    pub fn execute_lasers(&mut self) {
        // the code for lasers and robot lasers is mostly the same, but with one difference:
        // - for robot lasers, we can't hit the tile we're starting on (you can't shoot yourself),
        //   so we start the loop with "incrementing" bullet position (incl. wall checks)
        // - for map lasers, the position increment is moved to the end of the loop, as we might
        //   already hit a robot on the tile we're shooting from
        let map = &self.game.upgrade().unwrap().map;
        let mut animations = Vec::new();
        for (start_pos, bullet_dir) in &map.lasers {
            let mut bullet_pos = *start_pos;
            let mut tile = map.tiles.get(bullet_pos).unwrap();
            'map_bullet_flight: loop {
                for player in &mut self.players {
                    if player.public_state.position == bullet_pos {
                        player.draw_spam();
                        animations.push(Animation::BulletFlight {
                            from: *start_pos,
                            to: bullet_pos,
                            direction: *bullet_dir,
                            is_from_tank: false,
                        });
                        break 'map_bullet_flight;
                    }
                }
                // wall on the tile we're leaving?
                if tile.walls.get(*bullet_dir) {
                    break;
                }
                bullet_pos = bullet_pos.moved_in_direction(*bullet_dir);
                tile = match map.tiles.get(bullet_pos) {
                    // out of map
                    None => break,
                    Some(t) => t,
                };
                // wall on the tile we're entering?
                if tile.walls.get(bullet_dir.rotated().rotated()) {
                    break;
                }
            }
        }
        let bullet_starts: Vec<_> = self
            .players
            .iter()
            .filter_map(|p| {
                (!p.public_state.is_rebooting)
                    .then(|| (p.public_state.direction.into(), p.public_state.position))
            })
            .collect();
        for (direction, start_position) in bullet_starts {
            let mut bullet_pos = start_position;
            let mut tile = map.tiles.get(bullet_pos).unwrap();
            'robot_bullet_flight: loop {
                // wall on the tile we're leaving?
                if tile.walls.get(direction) {
                    break;
                }
                bullet_pos = bullet_pos.moved_in_direction(direction);
                tile = match map.tiles.get(bullet_pos) {
                    // out of map
                    None => break,
                    Some(t) => t,
                };
                // wall on the tile we're entering?
                if tile.walls.get(direction.rotated().rotated()) {
                    break;
                }
                for player2 in &mut self.players {
                    if player2.public_state.position == bullet_pos {
                        player2.draw_spam();
                        animations.push(Animation::BulletFlight {
                            from: start_position,
                            to: bullet_pos,
                            direction,
                            is_from_tank: true,
                        });
                        break 'robot_bullet_flight;
                    }
                }
            }
        }
        if !animations.is_empty() {
            self.send_animation_item(&animations, false);
        }
    }

    pub fn execute_checkpoints(&mut self) {
        let map = &self.game.upgrade().unwrap().map;
        for player_i in self.player_indices_by_priority() {
            let player = &mut self.players[player_i];
            if player.public_state.is_rebooting {
                continue;
            }
            if map.checkpoints.get(player.public_state.checkpoint)
                == Some(&player.public_state.position)
            {
                player.public_state.checkpoint += 1;
                if self.winner.is_none() && player.public_state.checkpoint == map.checkpoints.len()
                {
                    self.winner = Some(player_i);
                }
                self.send_animation_item(&[Animation::CheckpointVisited { player_i }], true);
            }
        }
    }
}
