use std::{
    collections::HashSet,
    iter::repeat,
    mem,
    sync::{Arc, Weak},
    time::Duration,
};

use futures::{
    future::{join_all, BoxFuture},
    FutureExt,
};
use rand::{prelude::SliceRandom, thread_rng};
use roborally_structs::{
    card::Card,
    game_map::GameMap,
    game_state::{GamePhaseView, PlayerGameStateView, PlayerPublicState, RegisterMovePhase},
    logging::{debug, info},
    position::{Direction, Position},
    tile_type::{BeltEnd, TileType},
    transport::ServerMessage,
};
use tokio::{sync::RwLock, time::sleep};

use crate::game_connection::PlayerConnection;

#[derive(Clone, Copy, Debug)]
pub struct DamagePiles {
    pub spam: u8,
    pub worm: u8,
    pub virus: u8,
    pub trojan: u8,
}

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
                direction: spawn_point.1,
                checkpoint: 0,
                is_rebooting: false,
            },
            draw_pile: Vec::new(),
            hand: Vec::new(),
            discard_pile: START_CARDS.into(),
            connected: Weak::new(),
        };
        p.hand = p.draw_n(9);
        p
    }

    async fn send_message(&self, msg: ServerMessage) {
        if let Some(conn) = self.connected.upgrade() {
            conn.socket.write().await.send_message(msg).await;
        }
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

    pub fn draw_spam(&mut self, damage_piles: &mut DamagePiles) {
        if damage_piles.spam > 0 {
            damage_piles.spam -= 1;
            self.discard_pile.push(Card::SPAM);
        } else {
            todo!()
        }
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

#[derive(PartialEq, Eq)]
enum MoveResult {
    DidntMove,
    Moved { rebooted: bool },
}

#[derive(Debug)]
#[non_exhaustive]
pub struct Game {
    pub map: GameMap,
    pub players: Vec<Player>,
    pub phase: GamePhase,
    pub name: String,
    pub damage_piles: DamagePiles,
}

impl Game {
    pub fn get_state_for_player(&self, seat: usize) -> PlayerGameStateView {
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
                .map(|p| p.connected.upgrade().map(|c| c.name.clone()))
                .collect(),
        )
    }

    pub fn new(map: GameMap, players_n: usize, name: String) -> Result<Self, String> {
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
            name,
            damage_piles: DamagePiles {
                spam: 30,
                worm: 15,
                virus: 15,
                trojan: 15,
            },
        })
    }

    pub async fn notify_update(&self) {
        let futures = self.players.iter().enumerate().map(|(i, player)| {
            player.send_message(ServerMessage::SetState(self.get_state_for_player(i)))
        });
        join_all(futures).await;
    }

    fn force_move_to(&mut self, player_i: usize, pos: Position, pushing_direction: Direction) {
        if !self
            .map
            .tiles
            .get(pos.x, pos.y)
            .is_some_and(|tile| tile.typ != TileType::Void)
        {
            panic!("Infinite reboot cycle entered");
        }
        let need_move: Vec<_> = self
            .players
            .iter()
            .enumerate()
            .filter(|(_, p)| p.public_state.position == pos)
            .map(|(p_i, _)| p_i)
            .collect();
        for player2_i in need_move {
            self.force_move_to(player2_i, pushing_direction.apply(&pos), pushing_direction);
        }
        self.players
            .get_mut(player_i)
            .unwrap()
            .public_state
            .position = pos;
    }

    fn reboot(&mut self, player_i: usize) {
        let reboot_token = self.map.reboot_token;
        let player_state = &mut self.players.get_mut(player_i).unwrap().public_state;
        player_state.direction = reboot_token.1;
        player_state.is_rebooting = true;
        // temporarily move them away, to prevent collisions with players pushed during reboot
        player_state.position.x = usize::MAX;
        self.force_move_to(player_i, reboot_token.0, reboot_token.1);
    }

    /// returns value:
    /// None => failed to move
    /// Some(vec) => moved, vec contains `player_i` of each player that was moved (in case of pushing train)
    fn mov(&mut self, player_i: usize, push: bool, dir_opt: Option<Direction>) -> MoveResult {
        debug!("Attempting to move player {}", player_i);
        let player = self.players.get(player_i).unwrap();
        let origin_pos = player.public_state.position;
        let target_pos;
        // fall through or break 'checks => can move
        let should_reboot = 'checks: {
            let Some(origin_tile) = self.map.tiles.get(origin_pos.x, origin_pos.y) else {
                return MoveResult::DidntMove;
            };
            let direction = dir_opt.unwrap_or(player.public_state.direction);
            if origin_tile.walls.get(&direction) {
                debug!("There's a wall on source tile");
                return MoveResult::DidntMove;
            }
            target_pos = direction.apply(&origin_pos);
            let Some(target_tile) = self.map.tiles.get(target_pos.x, target_pos.y) else {
                debug!("Falling out from map");
                // falling into void
                break 'checks true;
            };
            if target_tile.walls.get(&direction.rotated().rotated()) {
                debug!("There's a wall on target tile");
                return MoveResult::DidntMove;
            }
            if target_tile.typ == TileType::Void {
                debug!("Falling into a void");
                break 'checks true;
            }
            // this separate extraction into a Vec is necessary to satisfy borrow rules
            let players_in_way: Vec<_> = self
                .players
                .iter()
                .enumerate()
                .filter(|(i, player2)| {
                    *i != player_i && player2.public_state.position == target_pos
                })
                .map(|(i, _)| i)
                .collect();
            assert!(
                players_in_way.len() <= 1,
                "Unexpected: more than 1 player on tile"
            );
            if let Some(player2_i) = players_in_way.first() {
                debug!("There's a player in the way");
                if !push {
                    debug!("Can't push, aborting");
                    return MoveResult::DidntMove;
                }
                match self.mov(*player2_i, true, Some(direction)) {
                    MoveResult::DidntMove => return MoveResult::DidntMove,
                    MoveResult::Moved { rebooted: _ } => {}
                }
            }
            false
        };
        debug!("Moving");
        if should_reboot {
            self.reboot(player_i);
        } else {
            self.players
                .get_mut(player_i)
                .unwrap()
                .public_state
                .position = target_pos;
        }
        MoveResult::Moved {
            rebooted: should_reboot,
        }
    }

    pub async fn program(&mut self, seat: usize, cards: [Card; 5]) -> Result<(), String> {
        let player = self.players.get_mut(seat).unwrap();
        let GamePhase::Programming(vec) = &mut self.phase else {
            return Err("Programming phase isn't active right now".to_owned());
        };
        if *cards.first().unwrap() == Card::Again {
            return Err("Can't program Again in first slot".to_owned());
        }
        let my_programmed_ref = match vec.get_mut(seat).unwrap() {
            Some(_) => {
                return Err("You have already programmed your cards for this round".to_owned());
            }
            x @ None => x,
        };
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
        Ok(())
    }
}

#[derive(PartialEq, Eq)]
struct AntennaDist {
    me: Position,
    antenna: Position,
}

impl AntennaDist {
    const fn dist(&self) -> usize {
        usize::abs_diff(self.antenna.x, self.me.x) + usize::abs_diff(self.antenna.y, self.me.y)
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
impl PartialOrd for AntennaDist {
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
impl Ord for AntennaDist {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.partial_cmp(other).unwrap()
    }
}

fn execute_card(
    mut game_arc: Arc<RwLock<Game>>,
    player_i: usize,
    register_i: usize,
) -> BoxFuture<'static, ()> {
    async move {
        use Card::*;
        let mut guard = game_arc.write().await;
        let game = &mut *guard;
        let GamePhase::Moving { cards, .. } = &mut game.phase
    else {
        panic!("Invalid state")
    };
        let card = cards
            .get_mut(player_i)
            .unwrap()
            .get_mut(register_i)
            .unwrap();
        let player = game.players.get_mut(player_i).unwrap();
        debug!("Executing {:?} for {}", &card, player_i);

        match card {
            SPAM => {
                game.damage_piles.spam += 1;
                *card = player.draw_one();
                drop(guard);
                notify_sleep(&mut game_arc).await;
                execute_card(game_arc, player_i, register_i).await;
            }
            Worm => {
                game.damage_piles.worm += 1;
                // even though the card won't be played, we need to set it to move the worm out of the deck
                *card = player.draw_one();
                game.reboot(player_i);
                drop(guard);
                notify_sleep(&mut game_arc).await;
            }
            Virus => todo!(),

            Trojan => {
                game.damage_piles.trojan += 1;
                player.draw_spam(&mut game.damage_piles);
                player.draw_spam(&mut game.damage_piles);
                *card = player.draw_one();
                drop(guard);
                notify_sleep(&mut game_arc).await;
                execute_card(game_arc, player_i, register_i).await;
            }
            Move1 => {
                game.mov(player_i, true, None);
                drop(guard);
                notify_sleep(&mut game_arc).await;
            }
            Move2 => {
                let moved = game.mov(player_i, true, None);
                drop(guard);
                notify_sleep(&mut game_arc).await;
                if moved == (MoveResult::Moved { rebooted: false }) {
                    game_arc.write().await.mov(player_i, true, None);
                    notify_sleep(&mut game_arc).await;
                }
            }
            Move3 => {
                let moved = game.mov(player_i, true, None);
                drop(guard);
                notify_sleep(&mut game_arc).await;
                if moved == (MoveResult::Moved { rebooted: false }) {
                    let moved2 = game_arc.write().await.mov(player_i, true, None);
                    notify_sleep(&mut game_arc).await;
                    if moved2 == (MoveResult::Moved { rebooted: false }) {
                        game_arc.write().await.mov(player_i, true, None);
                        notify_sleep(&mut game_arc).await;
                    }
                }
            }
            Reverse1 => {
                let dir = player.public_state.direction.rotated().rotated();
                game.mov(player_i, true, Some(dir));
                drop(guard);
                notify_sleep(&mut game_arc).await;
            }
            TurnRight => {
                player.public_state.direction = player.public_state.direction.rotated();
                drop(guard);
                notify_sleep(&mut game_arc).await;
            }
            TurnLeft => {
                player.public_state.direction =
                    player.public_state.direction.rotated().rotated().rotated();
                drop(guard);
                notify_sleep(&mut game_arc).await;
            }
            UTurn => {
                player.public_state.direction = player.public_state.direction.rotated().rotated();
                drop(guard);
                notify_sleep(&mut game_arc).await;
            }
            Again => {
                drop(guard);
                execute_card(game_arc, player_i, register_i - 1).await;
            }
        }
    }
    .boxed()
}

pub async fn run_moving_phase(mut game_arc: Arc<RwLock<Game>>) {
    use RegisterMovePhase::*;
    {
        let mut guard = game_arc.write().await;
        let game = &mut *guard;
        if let GamePhase::Programming(cards) = &game.phase {
            game.phase = GamePhase::Moving {
                cards: cards.iter().map(|c| c.unwrap()).collect(),
                register: 0,
                register_phase: RegisterMovePhase::PlayerCards,
            };
        }
    }
    loop {
        let register;
        let register_phase;
        let player_i_sorted_by_priority;
        {
            let game = game_arc.read().await;
            match &game.phase {
                GamePhase::Moving {
                    register: reg,
                    register_phase: phase,
                    ..
                } => {
                    register = *reg;
                    register_phase = *phase;
                }
                _ => panic!("Invalid state"),
            };
            let mut active_players: Vec<usize> = game
                .players
                .iter()
                .enumerate()
                .filter(|(_, p)| !p.public_state.is_rebooting)
                .map(|(i, _)| i)
                .collect();
            active_players.sort_by_key(|i| AntennaDist {
                me: unsafe { game.players.get_unchecked(*i) }
                    .public_state
                    .position,
                antenna: game.map.antenna,
            });
            player_i_sorted_by_priority = active_players;
        }
        debug!("Executing phase {}.{:?}", register, register_phase);
        let next_register_phase = match register_phase {
            PlayerCards => {
                notify_sleep(&mut game_arc).await;
                for player_i in player_i_sorted_by_priority {
                    execute_card(Arc::clone(&game_arc), player_i, register).await;
                }
                FastBelts
            }
            FastBelts => {
                for player_i in player_i_sorted_by_priority {
                    for _ in 0..2 {
                        let mut game = game_arc.write().await;
                        let pos = game.players.get(player_i).unwrap().public_state.position;
                        if let TileType::Belt(true, dir, end) =
                            game.map.tiles.get(pos.x, pos.y).unwrap().typ
                        {
                            debug!("Moving player {} on a fast belt", player_i);
                            let moved = game.mov(player_i, false, Some(dir));
                            if (moved == MoveResult::Moved { rebooted: false }) {
                                match end {
                                    BeltEnd::Straight => {}
                                    BeltEnd::TurnLeft => {
                                        let player_public_state = &mut game
                                            .players
                                            .get_mut(player_i)
                                            .unwrap()
                                            .public_state;
                                        player_public_state.direction = player_public_state
                                            .direction
                                            .rotated()
                                            .rotated()
                                            .rotated();
                                    }
                                    BeltEnd::TurnRight => {
                                        let player_public_state = &mut game
                                            .players
                                            .get_mut(player_i)
                                            .unwrap()
                                            .public_state;
                                        player_public_state.direction =
                                            player_public_state.direction.rotated();
                                    }
                                }
                            }
                            drop(game);
                            notify_sleep(&mut game_arc).await;
                        }
                    }
                }
                SlowBelts
            }
            SlowBelts => {
                for player_i in player_i_sorted_by_priority {
                    let mut game = game_arc.write().await;
                    let pos = game.players.get(player_i).unwrap().public_state.position;
                    if let TileType::Belt(false, dir, end) =
                        game.map.tiles.get(pos.x, pos.y).unwrap().typ
                    {
                        debug!("Moving player {} on a slow belt", player_i);
                        let moved = game.mov(player_i, false, Some(dir));
                        if (moved == MoveResult::Moved { rebooted: false }) {
                            match end {
                                BeltEnd::Straight => {}
                                BeltEnd::TurnLeft => {
                                    let player_public_state =
                                        &mut game.players.get_mut(player_i).unwrap().public_state;
                                    player_public_state.direction =
                                        player_public_state.direction.rotated().rotated().rotated();
                                }
                                BeltEnd::TurnRight => {
                                    let player_public_state =
                                        &mut game.players.get_mut(player_i).unwrap().public_state;
                                    player_public_state.direction =
                                        player_public_state.direction.rotated();
                                }
                            }
                        }
                        drop(game);
                        notify_sleep(&mut game_arc).await;
                    }
                }
                PushPanels
            }
            PushPanels => {
                for player_i in player_i_sorted_by_priority {
                    let mut game = game_arc.write().await;
                    let pos = game.players.get(player_i).unwrap().public_state.position;
                    if let TileType::PushPanel(dir, active) =
                        game.map.tiles.get(pos.x, pos.y).unwrap().typ
                    {
                        if *active.get(register).unwrap() {
                            debug!("Moving player {} from a push panel", player_i);
                            game.mov(player_i, true, Some(dir));
                            drop(game);
                            notify_sleep(&mut game_arc).await;
                        }
                    }
                }
                Rotations
            }
            Rotations => {
                let mut guard = game_arc.write().await;
                // explicitely separating guard to satisfy borrow rules - can't borrow from guard twice
                // at once (deref), but can deref once and then borrow different fields
                let game: &mut Game = &mut *guard;
                let mut any_rotated = false;
                for player_i in player_i_sorted_by_priority {
                    let player_state = &mut game.players.get_mut(player_i).unwrap().public_state;
                    if let TileType::Rotation(is_cw) = game
                        .map
                        .tiles
                        .get(player_state.position.x, player_state.position.y)
                        .unwrap()
                        .typ
                    {
                        let dir = &mut player_state.direction;
                        *dir = if is_cw {
                            dir.rotated()
                        } else {
                            dir.rotated().rotated().rotated()
                        };
                        any_rotated = true;
                    }
                }
                drop(guard);
                if any_rotated {
                    notify_sleep(&mut game_arc).await;
                }
                Lasers
            }
            Lasers => {
                // todo lasers
                RobotLasers
            }
            RobotLasers => {
                //todo robot lasers
                Checkpoints
            }
            Checkpoints => {
                let mut winner = None;

                let mut guard = game_arc.write().await;
                let game: &mut Game = &mut *guard;
                for player_i in player_i_sorted_by_priority {
                    let player = game.players.get_mut(player_i).unwrap();
                    if *game
                        .map
                        .checkpoints
                        .get(player.public_state.checkpoint)
                        .unwrap()
                        == player.public_state.position
                    {
                        player.public_state.checkpoint += 1;
                        if winner.is_none()
                            && player.public_state.checkpoint == game.map.checkpoints.len()
                        {
                            winner = Some(player_i);
                        }
                    }
                }

                drop(guard);
                notify_sleep(&mut game_arc).await;

                {
                    if let Some(player_i) = winner {
                        {
                            #[allow(clippy::shadow_unrelated)]
                            let mut game = game_arc.write().await;
                            info!(
                                "Game won by {}",
                                game.players
                                    .get(player_i)
                                    .unwrap()
                                    .connected
                                    .upgrade()
                                    .map_or_else(
                                        || "<disconnected player>".to_owned(),
                                        |p| p.name.clone()
                                    )
                            );
                            game.phase = GamePhase::HasWinner(player_i);
                            game.notify_update().await;
                            drop(game);
                            return;
                        }
                    }
                }

                if register < 4 {
                    #[allow(clippy::shadow_unrelated)]
                    let mut game = game_arc.write().await;
                    #[allow(clippy::shadow_unrelated)]
                    if let GamePhase::Moving { register, .. } = &mut game.phase {
                        *register += 1;
                    } else {
                        panic!("Invalid state");
                    }
                    drop(game);
                    PlayerCards
                } else {
                    #[allow(clippy::shadow_unrelated)]
                    let mut game = game_arc.write().await;
                    let cards = match &game.phase {
                        GamePhase::Moving { cards, .. } => cards.clone(),
                        _ => panic!("Invalid state"),
                    };
                    for (player, player_programmed) in game.players.iter_mut().zip(cards.iter()) {
                        player.discard_pile.extend(player_programmed);
                        player.discard_pile.extend(mem::take(&mut player.hand));
                        player.hand = player.draw_n(9);
                    }
                    game.phase =
                        GamePhase::Programming(repeat(None).take(game.players.len()).collect());
                    debug!("Set game phase back to programming");
                    game.notify_update().await;
                    return;
                }
            }
        };
        let mut game = game_arc.write().await;
        #[allow(clippy::shadow_unrelated)]
        match &mut game.phase {
            GamePhase::Moving { register_phase, .. } => *register_phase = next_register_phase,
            _ => panic!("Invalid state"),
        }
    }
}

/// Asking for a mutable `game_arc` reference to help check that it isn't borrowed anywhere
async fn notify_sleep(game_arc: &mut Arc<RwLock<Game>>) {
    game_arc.read().await.notify_update().await;
    sleep(Duration::from_secs(1)).await;
}
