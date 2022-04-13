use std::{
    collections::{HashMap, HashSet},
    iter::repeat,
    mem,
    sync::Weak,
};

use futures::future::join_all;
use rand::{prelude::SliceRandom, thread_rng};
use roborally_structs::{
    animations::Animation,
    card::Card,
    game_map::GameMap,
    game_state::{GamePhaseView, PlayerGameStateView, PlayerPublicState, RegisterMovePhase},
    logging::{debug, info},
    position::{ContinuousDirection, Direction, Position},
    tile::Tile,
    tile_type::TileType,
    transport::{ServerMessage, StateArrayItem},
};

use crate::game_connection::PlayerConnection;

#[derive(Debug)]
pub struct Player {
    public_state: PlayerPublicState,
    draw_pile: Vec<Card>,
    hand: Vec<Card>,
    discard_pile: Vec<Card>,
    pub connected: Weak<PlayerConnection>,
}

const START_CARDS: [Card; 20] = {
    use Card::*;
    [
        Move1, Move1, Move1, Move1, Move1, Move2, Move2, Move2, Move2, Move3, Reverse1, Reverse1,
        TurnRight, TurnRight, TurnRight, TurnLeft, TurnLeft, TurnLeft, UTurn, Again,
    ]
};

impl Player {
    pub fn init(spawn_point: &(Position, Direction)) -> Self {
        let mut p = Self {
            public_state: PlayerPublicState {
                position: spawn_point.0,
                direction: spawn_point.1.to_continuous(),
                checkpoint: 0,
                is_rebooting: false,
                is_hidden: false,
            },
            draw_pile: Vec::new(),
            hand: Vec::new(),
            discard_pile: START_CARDS.into(),
            connected: Weak::new(),
        };
        p.hand = p.draw_n(9);
        p
    }

    pub fn draw_one(&mut self) -> Card {
        if let Some(c) = self.draw_pile.pop() {
            c
        } else {
            self.draw_pile = mem::take(&mut self.discard_pile);
            self.draw_pile.shuffle(&mut thread_rng());
            self.draw_pile.pop().unwrap()
        }
    }

    pub fn draw_n(&mut self, n: usize) -> Vec<Card> {
        if self.draw_pile.len() >= n {
            self.draw_pile.split_off(self.draw_pile.len() - n)
        } else {
            // Vec::new() -> discard_pile -> draw_pile -> out
            let mut out = mem::replace(&mut self.draw_pile, mem::take(&mut self.discard_pile));
            self.draw_pile.shuffle(&mut thread_rng());
            out.extend(
                self.draw_pile
                    .split_off(self.draw_pile.len() - (n - out.len())),
            );
            out
        }
    }

    pub fn draw_spam(&mut self) {
        self.discard_pile.push(Card::SPAM);
    }
}

#[derive(Debug)]
pub enum GamePhase {
    Programming(Vec<Option<[Card; 5]>>),
    Moving {
        cards: Vec<[Card; 5]>,
        register: usize,
        register_phase: RegisterMovePhase,
    },
    HasWinner(usize),
}

#[derive(Debug)]
pub struct Game {
    pub map: GameMap,
    pub players: Vec<Player>,
    pub phase: GamePhase,
}

impl Game {
    pub fn get_state_for_player(&mut self, seat: usize) -> PlayerGameStateView {
        let this_player_state = self.players.get(seat).unwrap();
        let phase: GamePhaseView = match &self.phase {
            GamePhase::Moving {
                cards,
                register,
                register_phase,
            } => GamePhaseView::Moving {
                cards: cards
                    .iter()
                    .map(|card_array| *card_array.get(*register).unwrap())
                    .collect(),
                my_registers: *cards.get(seat).unwrap(),
                register: *register,
                register_phase: *register_phase,
            },
            GamePhase::Programming(programmed) => GamePhaseView::Programming {
                ready: programmed.iter().map(Option::is_some).collect(),
                my_cards: *programmed.get(seat).unwrap(),
            },
            GamePhase::HasWinner(player_i) => GamePhaseView::HasWinner(*player_i),
        };
        PlayerGameStateView::new(
            self.players.iter().map(|p| p.public_state).collect(),
            phase,
            this_player_state.hand.clone(),
            self.players
                .iter()
                .map(|p| p.connected.upgrade().map(|c| c.player_name.clone()))
                .collect(),
        )
    }

    pub fn new(map: GameMap, players_n: usize) -> Result<Self, String> {
        if map.spawn_points.len() < players_n {
            return Err("Not enough spawn points on map".to_owned());
        }
        let mut spawn_points = map.spawn_points.clone();
        let (shuffled_spawn_points, _) = spawn_points.partial_shuffle(&mut thread_rng(), players_n);
        let players: Vec<Player> = shuffled_spawn_points.iter().map(Player::init).collect();
        Ok(Self {
            map,
            players,
            phase: GamePhase::Programming(repeat(None).take(players_n).collect()),
        })
    }

    pub async fn send_single_update(&mut self) {
        // we need a separate map and collect to satisfy `self` borrow rules
        #[allow(clippy::needless_collect)]
        let connections: Vec<_> = self
            .players
            .iter()
            .enumerate()
            .filter_map(|(i, player)| player.connected.upgrade().map(|conn| (i, conn)))
            .collect();
        let futures = connections
            .into_iter()
            // two separate closures to avoid using self in async (again, borrow rules)
            .map(|(player_i, conn)| (conn, self.get_state_for_player(player_i)))
            .map(async move |(conn, msg)| {
                conn.socket
                    .write()
                    .await
                    .send_message(ServerMessage::State(msg))
                    .await;
            });
        join_all(futures).await;
    }

    pub async fn program(&mut self, seat: usize, cards: [Card; 5]) -> Result<(), String> {
        let player = self.players.get_mut(seat).unwrap();
        let my_programmed_ref = match &mut self.phase {
            GamePhase::Programming(vec) => &mut vec[seat],
            _ => return Err("Programming phase isn't active right now".to_owned()),
        };
        if my_programmed_ref.is_some() {
            return Err("You have already programmed your cards for this round".to_owned());
        }
        let mut used_hand_indexes = HashSet::new();
        'outer: for picked_card in cards {
            for (i, hand_card) in player.hand.iter().enumerate() {
                if *hand_card == picked_card && !used_hand_indexes.contains(&i) {
                    used_hand_indexes.insert(i);
                    continue 'outer;
                }
            }
            // did not find this card (unused) in hand
            return Err(format!(
                "No cheating! {:?} isn't in your hand (enough times)",
                picked_card
            ));
        }
        *my_programmed_ref = Some(cards);

        let mut i = 0;
        player.hand.retain(move |_| {
            let res = !used_hand_indexes.contains(&i);
            i += 1;
            res
        });
        self.send_single_update().await;

        if let GamePhase::Programming(vec) = &self.phase {
            if vec.iter().all(std::option::Option::is_some) {
                self.phase = GamePhase::Moving {
                    cards: vec.iter().map(|c| c.unwrap()).collect(),
                    register: 0,
                    register_phase: RegisterMovePhase::PlayerCards,
                };
                self.run_moving_phase().await;
            }
        }

        Ok(())
    }

    async fn run_moving_phase(&mut self) {
        let states = (0..self.players.len()).map(|_| Vec::new()).collect();
        let mut manager = MovingPhaseManager {
            game: self,
            states,
            reboot_queue: Vec::new(),
        };
        manager.run();
        let futures = manager.states.into_iter().zip(self.players.iter()).map(
            async move |(state_array, player)| {
                if let Some(conn) = player.connected.upgrade() {
                    conn.socket
                        .write()
                        .await
                        .send_message(ServerMessage::AnimatedStates(state_array))
                        .await;
                }
            },
        );
        join_all(futures).await;
        self.send_single_update().await;
    }
}

enum QueueUpdateType {
    StateOnly,
    AnimationsOnly(Vec<Animation>),
}

struct MovingPhaseManager<'a> {
    game: &'a mut Game,
    states: Vec<Vec<StateArrayItem>>,
    reboot_queue: Vec<usize>,
}

impl<'a> MovingPhaseManager<'a> {
    fn queue_update(&mut self, update_type: &QueueUpdateType) {
        for (player_i, player_state_array) in self.states.iter_mut().enumerate() {
            player_state_array.push(match update_type {
                QueueUpdateType::StateOnly => {
                    StateArrayItem::new(Some(self.game.get_state_for_player(player_i)), Vec::new())
                }
                QueueUpdateType::AnimationsOnly(animations) => {
                    StateArrayItem::new(None, animations.clone())
                }
            });
        }
    }

    /// Moves player to given position. If needed, all players will be forcibly pushed away in the given direction.
    ///
    /// Forcibly pushing players away means that any walls in the way will be ignored.
    ///
    /// This method is used during rebooting - players already on the reboot token are pushed away. Original game maps
    /// are designed in such a way that pushing through a wall should never happen, but we need to take care of that
    /// edge-case.
    ///
    /// It is also possible that the "pushing train" of robots becomes so long, that the first one of them would fall
    /// into a void again. In that case, this method panics, poisoning the whole game.
    /// (TODO: find out what happens after that)
    fn force_move_to(&mut self, player_i: usize, pos: Position, pushing_direction: Direction) {
        if !self
            .game
            .map
            .tiles
            .get(pos)
            .is_some_and(|tile| tile.typ != TileType::Void)
        {
            panic!("Infinite reboot cycle entered");
        }
        let need_move: Vec<_> = self
            .game
            .players
            .iter()
            .enumerate()
            .filter(|(_, p)| p.public_state.position == pos)
            .map(|(p_i, _)| p_i)
            .collect();
        for player2_i in need_move {
            self.force_move_to(
                player2_i,
                pushing_direction.apply_to(&pos),
                pushing_direction,
            );
        }
        self.game
            .players
            .get_mut(player_i)
            .unwrap()
            .public_state
            .position = pos;
    }

    /// return value: if succeeded to free the tile
    fn mov(&mut self, player_i: usize, direction: Direction) -> bool {
        let player = self.game.players.get_mut(player_i).unwrap();
        let origin_pos = player.public_state.position;

        let Some(origin_tile) = self.game.map.tiles.get(origin_pos)
        else {
            // can't fail to free out-of-map (void) tile
            return true;
        };
        if origin_tile.typ == TileType::Void {
            // can't fail to free void tile, but don't move
            return true;
        }
        if origin_tile.walls.get(&direction) {
            // todo: failed move animation
            return false;
        }
        let target_pos = direction.apply_to(&origin_pos);
        let Some(target_tile) = self.game.map.tiles.get(target_pos)
        else {
            player.public_state.position = target_pos;
            self.reboot_queue.push(player_i);
            return true;
        };
        if target_tile.walls.get(&direction.rotated().rotated()) {
            // todo: failed move animation
            return false;
        }
        if target_tile.typ == TileType::Void {
            player.public_state.position = target_pos;
            self.reboot_queue.push(player_i);
            return true;
        }
        // this separate extraction into a Vec is necessary to satisfy borrow rules
        let players_in_way: Vec<_> = self
            .game
            .players
            .iter()
            .enumerate()
            .filter(|(i, player2)| *i != player_i && player2.public_state.position == target_pos)
            .map(|(i, _)| i)
            .collect();
        assert!(
            players_in_way.len() <= 1,
            "Unexpected: more than 1 player on tile"
        );
        if let Some(player2_i) = players_in_way.first() {
            if !self.mov(*player2_i, direction) {
                // todo: failed move animation
                return false;
            }
        }
        self.game.players[player_i].public_state.position = target_pos;
        true
    }

    fn update_with_reboots(&mut self) {
        for player_i in &self.reboot_queue {
            self.game.players[*player_i].public_state.is_hidden = true;
        }
        self.queue_update(&QueueUpdateType::StateOnly);

        let reboot_token = self.game.map.reboot_token;
        for player_i in mem::take(&mut self.reboot_queue) {
            let player = self.game.players.get_mut(player_i).unwrap();
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
            self.force_move_to(player_i, reboot_token.0, reboot_token.1);
            self.queue_update(&QueueUpdateType::StateOnly);
        }
    }

    fn execute_card(&mut self, player_i: usize, register_i: usize) {
        use Card::*;

        let GamePhase::Moving { cards, .. } = &mut self.game.phase
        else {
            unreachable!();
        };

        let card = &mut cards[player_i][register_i];
        let player = &mut self.game.players[player_i];
        debug!("Executing {:?} for {}", &card, player_i);

        match card {
            SPAM => {
                *card = player.draw_one();
                // to show the replaced card
                self.queue_update(&QueueUpdateType::StateOnly);
                self.execute_card(player_i, register_i);
            }
            mov @ (Move1 | Move2 | Move3 | Reverse1) => {
                let mut dir = player.public_state.direction.to_basic();
                if *mov == Reverse1 {
                    dir = dir.rotated().rotated();
                }
                let n = match mov {
                    Move1 | Reverse1 => 1,
                    Move2 => 2,
                    Move3 => 3,
                    _ => unreachable!(),
                };
                for _ in 0..n {
                    self.mov(player_i, dir);
                    self.update_with_reboots();
                    if self.game.players[player_i].public_state.is_rebooting {
                        break;
                    }
                }
            }
            TurnRight => {
                player.public_state.direction = player.public_state.direction.rotated();
                self.queue_update(&QueueUpdateType::StateOnly);
            }
            TurnLeft => {
                player.public_state.direction = player.public_state.direction.rotated_ccw();
                self.queue_update(&QueueUpdateType::StateOnly);
            }
            UTurn => {
                player.public_state.direction = player.public_state.direction.rotated().rotated();
                self.queue_update(&QueueUpdateType::StateOnly);
            }
            Again => {
                if register_i == 0 {
                    let replacement_card = player.draw_one();
                    player
                        .discard_pile
                        .push(mem::replace(card, replacement_card));
                    // show the replaced card
                    self.queue_update(&QueueUpdateType::StateOnly);
                    self.execute_card(player_i, register_i);
                } else {
                    self.execute_card(player_i, register_i - 1);
                }
            }
        }
    }

    /// Simulates the whole moving phase, populating `self.states` in the process
    ///
    /// This function must be called when `game.phase` is still [`GameState::Programming`]
    /// Panics if programming phase isn't active or some player doesn't have all cards programmed
    ///
    /// After this method returns, `game.phase` is set back to `Programming` (or eventualy `HasWinner`)
    fn run(&mut self) {
        use RegisterMovePhase::*;

        loop {
            self.queue_update(&QueueUpdateType::StateOnly);
            let GamePhase::Moving {register, register_phase, ..} = self.game.phase else {
                unreachable!();
            };
            let player_indexes_by_priority = {
                let mut indexes: Vec<usize> = (0..self.game.players.len()).collect();
                indexes.sort_by_key(|i| Priority {
                    me: self.game.players[*i].public_state.position,
                    antenna: self.game.map.antenna,
                });
                indexes
            };
            let next_register_phase = match register_phase {
                PlayerCards => {
                    for player_i in player_indexes_by_priority {
                        if !self
                            .game
                            .players
                            .get(player_i)
                            .unwrap()
                            .public_state
                            .is_rebooting
                        {
                            self.execute_card(player_i, register);
                        }
                    }
                    FastBelts
                }
                belts @ (FastBelts | SlowBelts) => {
                    let (n, expected_is_fast, next_phase) = match belts {
                        FastBelts => (2, true, SlowBelts),
                        SlowBelts => (1, false, PushPanels),
                        _ => unreachable!(),
                    };
                    for _ in 0..n {
                        // Mapping Position -> Indexes of all players on that tile (can't store players directly due to borrow rules)
                        let mut moved_positions: HashMap<
                            Position,
                            Vec<(usize, ContinuousDirection)>,
                        > = HashMap::new();
                        for (player_i, player) in self.game.players.iter_mut().enumerate() {
                            let player_pos = player.public_state.position;
                            let player_dir = player.public_state.direction;

                            let (position, direction) = if let Some(Tile {
                                typ: TileType::Belt(is_fast, dir),
                                walls,
                            }) = self.game.map.tiles.get(player_pos)
                            && *is_fast == expected_is_fast
                            && !walls.get(dir)
                            // is on belt and can leave current tile
                            {
                                let new_pos = dir.apply_to(&player_pos);
                                let new_tile = self.game.map.tiles.get(new_pos);
                                if new_tile.is_some_and(|t| t.walls.get(&dir.rotated().rotated())) {
                                    // can't enter target tile (wall)
                                    (player_pos, player_dir)
                                } else {
                                    // actually move, now just need to potentially rotate
                                    let new_dir = if let Some(Tile {
                                        typ: TileType::Belt(is_fast2, dir2),
                                        ..
                                    }) = new_tile
                                    && *is_fast2 == expected_is_fast
                                    {
                                        if *dir2 == dir.rotated() {
                                            player_dir.rotated()
                                        } else if *dir2 == dir.rotated_ccw() {
                                            player_dir.rotated_ccw()
                                        } else {
                                            player_dir
                                        }
                                    } else {
                                        player_dir
                                    };
                                    (new_pos, new_dir)
                                }
                            } else {
                                // not on belt or wall on current tile
                                (player_pos, player_dir)
                            };
                            moved_positions
                                .entry(position)
                                .or_default()
                                .push((player_i, direction));
                        }
                        loop {
                            let mut made_changes = false;
                            for (position, players) in mem::take(&mut moved_positions) {
                                if players.len() > 1
                                    // conflicts out of bounds or on void tiles don't matter
                                    && self
                                        .game
                                        .map
                                        .tiles
                                        .get(position)
                                        .is_some_and(|t| t.typ != TileType::Void)
                                {
                                    made_changes = true;
                                    // move all players on the conflicted tile to the original position
                                    // -> those that were attempted to be moved by a belt are reset, the rest stay
                                    for (player_i, _) in players {
                                        let player_state = self.game.players[player_i].public_state;
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
                        let mut to_reboot = Vec::new();
                        for (position, players) in moved_positions {
                            for (player_i, direction) in &players {
                                let player_state = &mut self.game.players[*player_i].public_state;
                                player_state.position = position;
                                player_state.direction = *direction;
                            }
                            if !self
                                .game
                                .map
                                .tiles
                                .get(position)
                                .is_some_and(|t| t.typ != TileType::Void)
                            {
                                to_reboot.extend(players.into_iter().map(|(i, _)| i));
                            }
                        }
                        to_reboot.sort_by_key(|i| Priority {
                            me: self.game.players[*i].public_state.position,
                            antenna: self.game.map.antenna,
                        });
                        self.reboot_queue.extend(to_reboot);
                        self.update_with_reboots();
                    }
                    next_phase
                }
                PushPanels => {
                    for player_i in player_indexes_by_priority {
                        let pos = self
                            .game
                            .players
                            .get(player_i)
                            .unwrap()
                            .public_state
                            .position;
                        if let TileType::PushPanel(dir, active) =
                            self.game.map.tiles.get(pos).unwrap().typ
                        {
                            if *active.get(register).unwrap() {
                                debug!("Moving player {} from a push panel", player_i);
                                self.mov(player_i, dir);
                                self.update_with_reboots();
                            }
                        }
                    }
                    Rotations
                }
                Rotations => {
                    let mut any_rotated = false;
                    for player_i in player_indexes_by_priority {
                        let player_state =
                            &mut self.game.players.get_mut(player_i).unwrap().public_state;
                        if let TileType::Rotation(is_cw) =
                            self.game.map.tiles.get(player_state.position).unwrap().typ
                        {
                            let dir = &mut player_state.direction;
                            *dir = if is_cw {
                                dir.rotated()
                            } else {
                                dir.rotated_ccw()
                            };
                            any_rotated = true;
                        }
                    }
                    if any_rotated {
                        self.queue_update(&QueueUpdateType::StateOnly);
                    }
                    Lasers
                }
                // the code for lasers and robot lasers is mostly the same, but with one difference:
                // - for robot lasers, we can't hit the tile we're starting on (you can't shoot yourself),
                //   so we start the loop with "incrementing" bullet position (incl. wall checks)
                // - for map lasers, the position increment is moved to the end of the loop, as we might
                //   already hit a robot on the tile we're shooting from
                Lasers => {
                    let mut animations = Vec::new();
                    for (start_pos, bullet_dir) in &self.game.map.lasers {
                        let mut bullet_pos = *start_pos;
                        let mut tile = *self.game.map.tiles.get(bullet_pos).unwrap();
                        'map_bullet_flight: loop {
                            for player2 in &mut self.game.players {
                                if player2.public_state.position == bullet_pos {
                                    debug!(
                                        "Laser shot player {:?}",
                                        player2.connected.upgrade().map(|c| c.player_name.clone())
                                    );
                                    player2.draw_spam();
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
                            if tile.walls.get(bullet_dir) {
                                break;
                            }
                            bullet_pos = bullet_dir.apply_to(&bullet_pos);
                            tile = match self.game.map.tiles.get(bullet_pos) {
                                // out of map
                                None => break,
                                Some(t) => *t,
                            };
                            // wall on the tile we're entering?
                            if tile.walls.get(&bullet_dir.rotated().rotated()) {
                                break;
                            }
                        }
                    }
                    for player_i in 0..self.game.players.len() {
                        let player_state = self.game.players.get(player_i).unwrap().public_state;
                        if player_state.is_rebooting {
                            // rebooting players don't shoot
                            continue;
                        }
                        let bullet_dir = player_state.direction.to_basic();
                        let start_pos = player_state.position;
                        let mut bullet_pos = start_pos;
                        let mut tile = *self.game.map.tiles.get(bullet_pos).unwrap();
                        'robot_bullet_flight: loop {
                            // wall on the tile we're leaving?
                            if tile.walls.get(&bullet_dir) {
                                break;
                            }
                            bullet_pos = bullet_dir.apply_to(&bullet_pos);
                            tile = match self.game.map.tiles.get(bullet_pos) {
                                // out of map
                                None => break,
                                Some(t) => *t,
                            };
                            // wall on the tile we're entering?
                            if tile.walls.get(&bullet_dir.rotated().rotated()) {
                                break;
                            }
                            for player2 in &mut self.game.players {
                                if player2.public_state.position == bullet_pos {
                                    debug!(
                                        "PLayer {} shot player {:?}",
                                        player_i,
                                        player2.connected.upgrade().map(|c| c.player_name.clone())
                                    );
                                    player2.draw_spam();
                                    animations.push(Animation::BulletFlight {
                                        from: start_pos,
                                        to: bullet_pos,
                                        direction: bullet_dir,
                                        is_from_tank: true,
                                    });
                                    break 'robot_bullet_flight;
                                }
                            }
                        }
                    }
                    self.queue_update(&QueueUpdateType::AnimationsOnly(animations));
                    Checkpoints
                }
                Checkpoints => {
                    let mut winner = None;
                    for player_i in 0..self.game.players.len() {
                        let player = self.game.players.get_mut(player_i).unwrap();
                        if self.game.map.checkpoints[player.public_state.checkpoint]
                            == player.public_state.position
                        {
                            // animation possibly here
                            player.public_state.checkpoint += 1;
                            if winner.is_none()
                                && player.public_state.checkpoint == self.game.map.checkpoints.len()
                            {
                                winner = Some(player_i);
                            }
                        }
                    }

                    if let Some(player_i) = winner {
                        {
                            info!(
                                "Game won by {}",
                                self.game
                                    .players
                                    .get(player_i)
                                    .unwrap()
                                    .connected
                                    .upgrade()
                                    .map_or_else(
                                        || "<disconnected player>".to_owned(),
                                        |p| p.player_name.clone()
                                    )
                            );
                            self.game.phase = GamePhase::HasWinner(player_i);
                            return;
                        }
                    }

                    if register < 4 {
                        #[allow(clippy::shadow_unrelated)]
                        if let GamePhase::Moving { register, .. } = &mut self.game.phase {
                            *register += 1;
                        } else {
                            unreachable!();
                        }
                        PlayerCards
                    } else {
                        let cards = match &self.game.phase {
                            GamePhase::Moving { cards, .. } => cards.clone(),
                            _ => unreachable!(),
                        };
                        for (player, player_programmed) in
                            self.game.players.iter_mut().zip(cards.iter())
                        {
                            player.discard_pile.extend(player_programmed);
                            player.discard_pile.extend(mem::take(&mut player.hand));
                            player.hand = player.draw_n(9);
                            player.public_state.is_rebooting = false;
                        }
                        self.game.phase = GamePhase::Programming(
                            repeat(None).take(self.game.players.len()).collect(),
                        );
                        return;
                    }
                }
            };
            match &mut self.game.phase {
                GamePhase::Moving { register_phase, .. } => *register_phase = next_register_phase,
                _ => unreachable!(),
            }
        }
    }
}
/// Small util that sorts by priority
#[derive(PartialEq, Eq)]
struct Priority {
    me: Position,
    antenna: Position,
}

impl Priority {
    const fn dist(&self) -> u16 {
        i16::abs_diff(self.antenna.x, self.me.x) + i16::abs_diff(self.antenna.y, self.me.y)
    }
    fn sortable_bearing(&self) -> f64 {
        let mut x = f64::atan2(
            self.antenna.x as f64 - self.me.x as f64,
            self.antenna.y as f64 - self.me.y as f64,
        );
        if x > 0.0 {
            x -= std::f64::consts::TAU;
        }
        -x
    }
}
impl PartialOrd for Priority {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        if self == other {
            return Some(core::cmp::Ordering::Equal);
        }

        match self.dist().partial_cmp(&other.dist()) {
            Some(core::cmp::Ordering::Equal) => {}
            ord => return ord,
        }
        self.sortable_bearing()
            .partial_cmp(&other.sortable_bearing())
    }
}
impl Ord for Priority {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.partial_cmp(other).unwrap()
    }
}
