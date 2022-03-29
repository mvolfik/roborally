use std::{
    collections::HashSet,
    iter::repeat,
    mem,
    ops::DerefMut,
    sync::{Arc, Weak},
    time::Duration,
};

use futures::future::join_all;
use rand::{prelude::SliceRandom, thread_rng};
use roborally_structs::{
    card::Card,
    game_map::GameMap,
    game_state::{GamePhaseView, PlayerGameStateView, PlayerPublicState, RegisterMovePhase},
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
            self.draw_pile = mem::replace(&mut self.discard_pile, Vec::new());
            self.draw_pile.shuffle(&mut thread_rng());
            self.draw_pile.pop().unwrap()
        }
    }

    pub fn draw_n(&mut self, n: usize) -> Vec<Card> {
        if self.draw_pile.len() >= n {
            self.draw_pile.split_off(self.draw_pile.len() - n)
        } else {
            // Vec::new() -> discard_pile -> draw_pile -> out
            let mut out = mem::replace(
                &mut self.draw_pile,
                mem::replace(&mut self.discard_pile, Vec::new()),
            );
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

#[derive(Debug)]
pub struct Game {
    pub map: GameMap,
    pub players: Vec<Player>,
    pub phase: GamePhase,
    pub name: String,
    pub damage_piles: DamagePiles,
    _prevent_construct: (),
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
            return Err("Not enough spawn points on map".to_string());
        }
        let mut spawn_points = map.spawn_points.clone();
        let (shuffled_spawn_points, _) = spawn_points.partial_shuffle(&mut thread_rng(), players_n);
        let players: Vec<Player> = shuffled_spawn_points
            .iter()
            .map(|sp| Player::init(sp))
            .collect();
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
            _prevent_construct: (),
        })
    }

    pub async fn notify_update(&self) {
        let futures = self.players.iter().enumerate().map(|(i, player)| {
            player.send_message(ServerMessage::SetState(self.get_state_for_player(i)))
        });
        join_all(futures).await;
    }

    fn mov(&mut self, player_i: usize, push: bool, direction: Option<Direction>) -> bool {
        let player = self.players.get_mut(player_i).unwrap();
        let origin_pos = player.public_state.position;
        let target_pos;
        // fall through or break 'checks => can move
        'checks: {
            let Some(origin_tile) = self.map.tiles.get(origin_pos.x, origin_pos.y) else {
                return false;
            };
            let direction = direction.unwrap_or(player.public_state.direction);
            if origin_tile.walls.get(&direction) {
                return false;
            }
            target_pos = direction.apply(&origin_pos);
            let Some(target_tile) = self.map.tiles.get(target_pos.x, target_pos.y) else {
                // falling into void
                break 'checks;
            };
            if origin_tile.walls.get(&direction.rotated().rotated()) {
                return false;
            }
            if target_tile.typ == TileType::Void {
                break 'checks;
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
            if (players_in_way.len() > 0 && !push)
                || players_in_way
                    .iter()
                    .any(|i| !self.mov(*i, true, Some(direction)))
            {
                return false;
            }
        }
        self.players
            .get_mut(player_i)
            .unwrap()
            .public_state
            .position = target_pos;
        true
    }

    fn execute_card(&mut self, player_i: usize, register_i: usize) {
        use Card::*;
        let player = self.players.get_mut(player_i).unwrap();
        let GamePhase::Moving { cards, .. } = &mut self.phase
        else {
            panic!("Invalid state")
        };
        let card = cards
            .get_mut(player_i)
            .unwrap()
            .get_mut(register_i)
            .unwrap();
        match card {
            SPAM => {
                self.damage_piles.spam += 1;
                *card = player.draw_one();
                self.execute_card(player_i, register_i);
            }
            Worm => {
                self.damage_piles.worm += 1;
                *card = player.draw_one();
                // move out of bounds = reboot
                player.public_state.position.x = usize::MAX;
            }
            Virus => todo!(),
            Trojan => {
                self.damage_piles.trojan += 1;
                player.draw_spam(&mut self.damage_piles);
                player.draw_spam(&mut self.damage_piles);
                *card = player.draw_one();
                self.execute_card(player_i, register_i);
            }
            Move1 => {
                self.mov(player_i, true, None);
            }
            Move2 => {
                let _ = self.mov(player_i, true, None) && self.mov(player_i, true, None);
            }
            Move3 => {
                let _ = self.mov(player_i, true, None)
                    && self.mov(player_i, true, None)
                    && self.mov(player_i, true, None);
            }
            Reverse1 => {
                let dir = player.public_state.direction.rotated().rotated();
                self.mov(player_i, true, Some(dir));
            }
            TurnRight => {
                player.public_state.direction = player.public_state.direction.rotated();
            }
            TurnLeft => {
                player.public_state.direction =
                    player.public_state.direction.rotated().rotated().rotated();
            }
            UTurn => {
                player.public_state.direction = player.public_state.direction.rotated().rotated();
            }
            Again => self.execute_card(player_i, register_i - 1),
        }
    }
    pub async fn program(&mut self, seat: usize, cards: [Card; 5]) -> Result<(), String> {
        let player = self.players.get_mut(seat).unwrap();
        let GamePhase::Programming(vec) = &mut self.phase else {
            return Err("Programming phase isn't active right now".to_string());
        };
        if *cards.first().unwrap() == Card::Again {
            return Err("Can't program Again in first slot".to_string());
        }
        let my_programmed_ref = match vec.get_mut(seat).unwrap() {
            Some(_) => {
                return Err("You have already programmed your cards for this round".to_string());
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
    fn dist(&self) -> usize {
        usize::abs_diff(self.antenna.x, self.me.x) + usize::abs_diff(self.antenna.y, self.me.y)
    }
    fn sortable_bearing(&self) -> f64 {
        let mut x = f64::atan2(
            self.antenna.x as f64 - self.me.x as f64,
            self.antenna.y as f64 - self.me.y as f64,
        );
        if x > 0.0 {
            x -= std::f64::consts::TAU
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

pub async fn run_moving_phase(game_arc: Arc<RwLock<Game>>, cards: Vec<[Card; 5]>) {
    use RegisterMovePhase::*;

    let sleep_after_move = || sleep(Duration::from_secs(3));

    game_arc.write().await.phase = GamePhase::Moving {
        cards,
        register: 0,
        register_phase: RegisterMovePhase::PlayerCards,
    };
    loop {
        let game = game_arc.read().await;
        let (cards, register, register_phase) = match &game.phase {
            GamePhase::Moving {
                cards,
                register,
                register_phase,
            } => (cards.clone(), *register, *register_phase),
            _ => panic!("Invalid state"),
        };
        let mut player_i_sorted_by_priority: Vec<usize> = game
            .players
            .iter()
            .enumerate()
            .filter(|(_, p)| p.public_state.is_rebooting)
            .map(|(i, _)| i)
            .collect();
        player_i_sorted_by_priority.sort_by_key(|i| AntennaDist {
            me: unsafe { game.players.get_unchecked(*i) }
                .public_state
                .position,
            antenna: game.map.antenna,
        });
        drop(game);
        let next_register_phase = match register_phase {
            PlayerCards => {
                for player_i in player_i_sorted_by_priority {
                    game_arc.write().await.execute_card(player_i, register);
                    game_arc.read().await.notify_update().await;
                    sleep_after_move().await;
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
                            let moved = game.mov(player_i, false, Some(dir));
                            if moved {
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
                                            .rotated()
                                    }
                                    BeltEnd::TurnRight => {
                                        let player_public_state = &mut game
                                            .players
                                            .get_mut(player_i)
                                            .unwrap()
                                            .public_state;
                                        player_public_state.direction =
                                            player_public_state.direction.rotated()
                                    }
                                }
                            }
                            drop(game);
                            game_arc.read().await.notify_update().await;
                            sleep_after_move().await;
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
                        let moved = game.mov(player_i, false, Some(dir));
                        if moved {
                            match end {
                                BeltEnd::Straight => {}
                                BeltEnd::TurnLeft => {
                                    let player_public_state =
                                        &mut game.players.get_mut(player_i).unwrap().public_state;
                                    player_public_state.direction =
                                        player_public_state.direction.rotated().rotated().rotated()
                                }
                                BeltEnd::TurnRight => {
                                    let player_public_state =
                                        &mut game.players.get_mut(player_i).unwrap().public_state;
                                    player_public_state.direction =
                                        player_public_state.direction.rotated()
                                }
                            }
                        }
                        drop(game);
                        game_arc.read().await.notify_update().await;
                        sleep_after_move().await;
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
                            game.mov(player_i, true, Some(dir));
                            drop(game);
                            game_arc.read().await.notify_update().await;
                            sleep_after_move().await;
                        }
                    }
                }
                Rotations
            }
            Rotations => {
                let mut guard = game_arc.write().await;
                // explicitely separating guard to satisfy borrow rules - can't borrow from guard twice
                // at once (deref), but can deref once and then borrow different fields
                let game: &mut Game = guard.deref_mut();
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
                    }
                }
                drop(guard);
                game_arc.read().await.notify_update().await;
                sleep_after_move().await;
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
                let mut guard = game_arc.write().await;
                let game: &mut Game = guard.deref_mut();
                let mut winner = None;
                for player_i in player_i_sorted_by_priority {
                    let player_state = &mut game.players.get_mut(player_i).unwrap().public_state;
                    if *game.map.checkpoints.get(player_state.checkpoint).unwrap()
                        == player_state.position
                    {
                        player_state.checkpoint += 1;
                        if winner.is_none() && player_state.checkpoint == game.map.checkpoints.len()
                        {
                            winner = Some(player_i)
                        }
                    }
                }
                drop(guard);
                game_arc.read().await.notify_update().await;
                sleep_after_move().await;

                let mut game = game_arc.write().await;
                if let Some(player_i) = winner {
                    game.phase = GamePhase::HasWinner(player_i);
                    drop(game);
                    game_arc.read().await.notify_update().await;
                    sleep(Duration::from_secs(60)).await;
                    return;
                }

                let mut game = game_arc.write().await;
                if register < 4 {
                    if let GamePhase::Moving { register, .. } = &mut game.phase {
                        *register += 1;
                    } else {
                        panic!("Invalid state");
                    }
                    PlayerCards
                } else {
                    for (player, player_programmed) in game.players.iter_mut().zip(cards.iter()) {
                        player.discard_pile.extend(player_programmed);
                        player
                            .discard_pile
                            .extend(mem::replace(&mut player.hand, Vec::new()));
                        player.hand = player.draw_n(9);
                    }
                    game.phase =
                        GamePhase::Programming(repeat(None).take(game.players.len()).collect());
                    drop(game);
                    game_arc.read().await.notify_update().await;
                    return;
                }
            }
        };
        let mut game = game_arc.write().await;
        // todo : if out of map, start reboot
        match &mut game.phase {
            GamePhase::Moving { register_phase, .. } => *register_phase = next_register_phase,
            _ => panic!("Invalid state"),
        }
    }
}
