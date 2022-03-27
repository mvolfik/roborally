use std::{collections::HashSet, iter::repeat, mem, time::Duration};

use actix::{Actor, AsyncContext, Context, Handler, Message, Response, WeakAddr};
use rand::{prelude::SliceRandom, thread_rng};
use roborally_structs::{
    card::Card,
    game_map::GameMap,
    game_state::{GamePhaseView, PlayerGameStateView, PlayerPublicState, RegisterMovePhase},
    position::{Direction, Position},
    tile_type::{BeltEnd, TileType},
    transport::ServerMessage,
};

use crate::game_connection::{CloseConnection, GameConnection, ServerActorMessage};

#[derive(Clone, Copy, Debug)]
pub struct DamagePiles {
    pub spam: u8,
    pub worm: u8,
    pub virus: u8,
    pub trojan: u8,
}

pub struct StateUpdate(pub PlayerGameStateView);
impl Message for StateUpdate {
    type Result = ();
}

#[derive(Debug)]
pub struct Player {
    public_state: PlayerPublicState,
    draw_pile: Vec<Card>,
    hand: Vec<Card>,
    discard_pile: Vec<Card>,
    pub connected: Option<(String, WeakAddr<GameConnection>)>,
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
            connected: None,
        };
        p.hand = p.draw_n(9);
        p
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
        register_phase_done: RegisterMovePhase,
    },
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

impl Actor for Game {
    type Context = Context<Self>;
}

impl Game {
    pub fn get_state_for_player(&self, seat: usize) -> PlayerGameStateView {
        let this_player_state = self.players.get(seat).unwrap();
        let phase: GamePhaseView = match &self.phase {
            GamePhase::Moving {
                cards,
                register,
                register_phase_done,
            } => GamePhaseView::Moving {
                cards: cards
                    .iter()
                    .map(|card_array| *card_array.get(*register).unwrap())
                    .collect(),
                register: *register,
                register_phase_done: *register_phase_done,
            },
            GamePhase::Programming(programmed) => GamePhaseView::Programming {
                ready: programmed.iter().map(Option::is_some).collect(),
                my_cards: *programmed.get(seat).unwrap(),
            },
        };
        PlayerGameStateView::new(
            self.players.iter().map(|p| p.public_state).collect(),
            phase,
            this_player_state.hand.clone(),
            self.players
                .iter()
                .map(|p| p.connected.as_ref().map(|c| c.0.clone()))
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

    fn notify_update(&self, ctx: &mut <Self as Actor>::Context) {
        for (i, player) in self.players.iter().enumerate() {
            send_message(
                player,
                ServerMessage::SetState(self.get_state_for_player(i)),
                ctx,
            )
        }
    }

    fn mov(&mut self, player_i: usize, push: bool, direction: Option<Direction>) -> bool {
        let player = self.players.get_mut(player_i).unwrap();
        let origin_pos = player.public_state.position;
        let Some(origin_tile) = self.map.tiles.get(origin_pos.x, origin_pos.y) else {
            return false;
        };
        let direction = direction.unwrap_or(player.public_state.direction);
        if origin_tile.walls.get(&direction) {
            return false;
        }
        let target_pos = direction.apply(&origin_pos);
        let Some(target_tile) = self.map.tiles.get(target_pos.x, target_pos.y) else {
            // falling into void
            player.public_state.position = target_pos;
            return true;
        };
        if origin_tile.walls.get(&direction.rotated().rotated()) {
            return false;
        }
        if target_tile.typ == TileType::Void {
            player.public_state.position = target_pos;
            return true;
        }
        // this separate extraction into a Vec is necessary to satisfy borrow rules
        let players_in_way: Vec<_> = self
            .players
            .iter()
            .enumerate()
            .filter(|(i, player2)| *i != player_i && player2.public_state.position == target_pos)
            .map(|(i, _)| i)
            .collect();
        let prevent_move = (players_in_way.len() > 0 && !push)
            || players_in_way
                .iter()
                .any(|i| !self.mov(*i, true, Some(direction)));

        if !prevent_move {
            self.players
                .get_mut(player_i)
                .unwrap()
                .public_state
                .position = target_pos;
            true
        } else {
            false
        }
    }

    fn execute_card(&mut self, player_i: usize, register_i: usize) {
        use Card::*;
        let player = self.players.get_mut(player_i).unwrap();
        let GamePhase::Moving{ cards, ..} = &mut self.phase  else {panic!("Invalid state")};
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
}

pub struct RequestConnect {
    pub name: String,
    pub seat: usize,
    pub weak_addr: WeakAddr<GameConnection>,
}
impl Message for RequestConnect {
    type Result = ();
}

impl Handler<RequestConnect> for Game {
    type Result = ();

    fn handle(
        &mut self,
        RequestConnect {
            name,
            seat,
            weak_addr,
        }: RequestConnect,
        _ctx: &mut Self::Context,
    ) -> Self::Result {
        let Some(addr) = weak_addr.upgrade() else {return};
        let Some(player) = self.players.get_mut(seat) else {
            addr.do_send(CloseConnection("There aren't that many seats".to_string()));
            return
        };
        if let Some((name, conn)) = player.connected.as_ref() {
            if conn.upgrade().is_some() {
                addr.do_send(CloseConnection(format!(
                    "Player '{}' is already connected to this seat",
                    name
                )));
                return;
            }
        }
        player.connected = Some((name, weak_addr));
        addr.do_send(ServerActorMessage(ServerMessage::InitInfo {
            map: self.map.clone(),
            state: self.get_state_for_player(seat),
        }));
    }
}

fn send_message(player: &Player, msg: ServerMessage, _ctx: &mut <Game as Actor>::Context) {
    if let Some((_, weak_addr)) = &player.connected {
        if let Some(addr) = weak_addr.upgrade() {
            addr.do_send(ServerActorMessage(msg))
        }
    }
}

pub struct Program(pub usize, pub [Card; 5]);
impl Message for Program {
    type Result = ();
}
impl Handler<Program> for Game {
    type Result = ();
    fn handle(&mut self, Program(seat, cards): Program, ctx: &mut Self::Context) {
        let player = self.players.get_mut(seat).unwrap();
        let GamePhase::Programming(vec) = &mut self.phase else {
            send_message(
                player,
                ServerMessage::Notice("Programming phase isn't active right now".to_string()),
                ctx,
            );
            return;
        };
        if *cards.first().unwrap() == Card::Again {
            send_message(
                player,
                ServerMessage::Notice("Can't program Again in first slot".to_string()),
                ctx,
            );
            return;
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
            send_message(
                player,
                ServerMessage::Notice(format!(
                    "No cheating! {:?} isn't in your hand (enough times)",
                    picked_card
                )),
                ctx,
            );
            return;
        }

        match vec.get_mut(seat).unwrap() {
            Some(_) => {
                send_message(
                    player,
                    ServerMessage::Notice(
                        "You have already programmed your cards for this round".to_string(),
                    ),
                    ctx,
                );
                return;
            }
            x @ None => *x = Some(cards),
        }

        let mut i = 0;
        player.hand.retain(move |_| {
            let res = !used_hand_indexes.contains(&i);
            i += 1;
            res
        });

        if vec.iter().all(Option::is_some) {
            self.phase = GamePhase::Moving {
                cards: vec.iter().map(|opt| opt.unwrap()).collect(),
                register: 0,
                register_phase_done: RegisterMovePhase::Started,
            };
            ctx.notify_later(Move, Duration::from_secs(1));
        }
        self.notify_update(ctx)
    }
}

pub struct RequestNameSeats;
impl Message for RequestNameSeats {
    type Result = (String, Vec<Option<String>>);
}
impl Handler<RequestNameSeats> for Game {
    type Result = Response<(String, Vec<Option<String>>)>;

    fn handle(&mut self, _msg: RequestNameSeats, _ctx: &mut Self::Context) -> Self::Result {
        Response::reply((
            self.name.clone(),
            self.players
                .iter()
                .map(|p| {
                    p.connected.as_ref().and_then(|(name, conn)| {
                        if conn.upgrade().is_some() {
                            Some(name.clone())
                        } else {
                            None
                        }
                    })
                })
                .collect(),
        ))
    }
}

pub struct Disconnect(pub usize);
impl Message for Disconnect {
    type Result = ();
}
impl Handler<Disconnect> for Game {
    type Result = ();

    fn handle(&mut self, Disconnect(seat): Disconnect, _ctx: &mut Self::Context) -> Self::Result {
        self.players.get_mut(seat).map(|p| p.connected = None);
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

struct Move;
impl Message for Move {
    type Result = ();
}
impl Handler<Move> for Game {
    type Result = ();

    fn handle(&mut self, _msg: Move, ctx: &mut Self::Context) -> Self::Result {
        use RegisterMovePhase::*;
        let GamePhase::Moving { register, register_phase_done, cards } = &mut self.phase else {panic!("Invalid state")};
        let mut player_i_sorted_by_priority: Vec<_> = (0..self.players.len()).collect();
        player_i_sorted_by_priority.sort_by_key(|i| AntennaDist {
            me: unsafe { self.players.get_unchecked(*i) }
                .public_state
                .position,
            antenna: self.map.antenna,
        });
        match register_phase_done {
            Started => {
                *register_phase_done = PlayerMove;
                let register_val = *register;
                for player_i in player_i_sorted_by_priority {
                    self.execute_card(player_i, register_val);
                }
            }
            PlayerMove => {
                *register_phase_done = FastBelts;
                for player_i in player_i_sorted_by_priority {
                    for _ in 0..2 {
                        let pos = self.players.get(player_i).unwrap().public_state.position;
                        if let TileType::Belt(true, dir, end) =
                            self.map.tiles.get(pos.x, pos.y).unwrap().typ
                        {
                            if self.mov(player_i, false, Some(dir)) {
                                match end {
                                    BeltEnd::Straight => {}
                                    BeltEnd::TurnLeft => {
                                        let player_public_state = &mut self
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
                                        let player_public_state = &mut self
                                            .players
                                            .get_mut(player_i)
                                            .unwrap()
                                            .public_state;
                                        player_public_state.direction =
                                            player_public_state.direction.rotated()
                                    }
                                }
                                continue;
                            }
                        }
                        break;
                    }
                }
            }
            FastBelts => {
                *register_phase_done = SlowBelts;
                for player_i in player_i_sorted_by_priority {
                    let pos = self.players.get(player_i).unwrap().public_state.position;
                    if let TileType::Belt(false, dir, end) =
                        self.map.tiles.get(pos.x, pos.y).unwrap().typ
                    {
                        if self.mov(player_i, false, Some(dir)) {
                            match end {
                                BeltEnd::Straight => {}
                                BeltEnd::TurnLeft => {
                                    let player_public_state =
                                        &mut self.players.get_mut(player_i).unwrap().public_state;
                                    player_public_state.direction =
                                        player_public_state.direction.rotated().rotated().rotated()
                                }
                                BeltEnd::TurnRight => {
                                    let player_public_state =
                                        &mut self.players.get_mut(player_i).unwrap().public_state;
                                    player_public_state.direction =
                                        player_public_state.direction.rotated()
                                }
                            }
                        }
                    }
                }
            }
            SlowBelts => {
                *register_phase_done = PushPanels;
                let register_val = *register;
                for player_i in player_i_sorted_by_priority {
                    let pos = self.players.get(player_i).unwrap().public_state.position;
                    if let TileType::PushPanel(dir, active) =
                        self.map.tiles.get(pos.x, pos.y).unwrap().typ
                    {
                        if *active.get(register_val).unwrap() {
                            self.mov(player_i, true, Some(dir));
                        }
                    }
                }
            }
            PushPanels => {
                *register_phase_done = Rotations;
                for player in self.players.iter_mut() {
                    let pos = player.public_state.position;
                    if let TileType::Rotation(is_cw) = self.map.tiles.get(pos.x, pos.y).unwrap().typ
                    {
                        let dir = &mut player.public_state.direction;
                        *dir = if is_cw {
                            dir.rotated()
                        } else {
                            dir.rotated().rotated().rotated()
                        };
                    }
                }
            }
            Rotations => {
                *register_phase_done = RobotLasers;
                // todo lasers
            }
            Lasers => todo!("robot lasers"),
            RobotLasers => {
                *register_phase_done = Checkpoints;
                for player in self.players.iter_mut() {
                    if *self
                        .map
                        .checkpoints
                        .get(player.public_state.checkpoint as usize)
                        .unwrap()
                        == player.public_state.position
                    {
                        player.public_state.checkpoint += 1
                    }
                }
            }
            Checkpoints => {
                if *register == 4 {
                    for (player, player_programmed) in self.players.iter_mut().zip(cards) {
                        player.discard_pile.extend(*player_programmed);
                        player
                            .discard_pile
                            .extend(mem::replace(&mut player.hand, Vec::new()));
                        player.hand = player.draw_n(9);
                    }
                    self.phase =
                        GamePhase::Programming(repeat(None).take(self.players.len()).collect());
                    self.notify_update(ctx);
                    return; // to prevent scheduling move again
                } else {
                    *register += 1;
                    *register_phase_done = Started;
                }
            }
        }
        self.notify_update(ctx);
        ctx.notify_later(Move, Duration::from_secs(1));
    }
}
