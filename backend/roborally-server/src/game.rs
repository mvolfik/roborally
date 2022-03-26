use std::{
    iter::repeat,
    sync::{Arc, Mutex},
};

use rand::{prelude::SliceRandom, thread_rng};
use roborally_structs::{
    game_map::GameMap,
    game_state::{GamePhaseView, PlayerGameStateView, PlayerPublicState},
    position::{Direction, Position},
    Card,
};

use crate::game_connection::GameConnection;

#[derive(Debug)]
pub struct Player {
    public_state: PlayerPublicState,
    draw_pile: Vec<Card>,
    hand: Vec<Card>,
    discard_pile: Vec<Card>,
    pub connected: Option<(String, ())>,
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
        Self {
            public_state: PlayerPublicState {
                position: spawn_point.0,
                direction: spawn_point.1,
                checkpoint: 0,
            },
            draw_pile: START_CARDS.into(),
            hand: Vec::new(),
            discard_pile: Vec::new(),
            connected: None,
        }
    }
}

#[derive(Debug)]
pub enum GamePhase {
    Programming(Vec<Option<[Card; 5]>>),
    Moving(Vec<[Card; 5]>),
}

#[derive(Debug)]
pub struct Game {
    pub map: GameMap,
    pub players: Vec<Player>,
    pub phase: GamePhase,
    pub name: String,
    _prevent_construct: (),
}

impl Game {
    pub fn get_state_for_player(&self, player_i: usize) -> Option<PlayerGameStateView> {
        let this_player_state = self.players.get(player_i)?;
        let phase: GamePhaseView = match &self.phase {
            GamePhase::Moving(cards) => GamePhaseView::Moving(cards.clone()),
            GamePhase::Programming(programmed) => {
                GamePhaseView::Programming(*programmed.get(player_i)?)
            }
        };
        Some(PlayerGameStateView::new(
            self.players.iter().map(|p| p.public_state).collect(),
            phase,
            this_player_state.hand.clone(),
        ))
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
            _prevent_construct: (),
        })
    }
}
