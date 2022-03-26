use crate::{position::Direction, position::Position, Card};
use serde::{Deserialize, Serialize};

#[cfg(feature = "client")]
use wasm_bindgen::prelude::wasm_bindgen;

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "server", derive(Serialize))]
#[cfg_attr(feature = "client", derive(Deserialize))]
pub enum GamePhaseView {
    /// Each player's programmed cards
    Moving(Vec<[Card; 5]>),
    /// My programmed cards, if any
    Programming(Option<[Card; 5]>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "server", derive(Serialize))]
#[cfg_attr(feature = "client", derive(Deserialize))]
pub struct PlayerPublicState {
    pub position: Position,
    pub direction: Direction,
    pub checkpoint: u8,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "server", derive(Serialize))]
#[cfg_attr(feature = "client", derive(Deserialize), wasm_bindgen)]
pub struct PlayerGameStateView {
    player_states: Vec<PlayerPublicState>,
    phase: GamePhaseView,
    hand: Vec<Card>,
}

#[cfg(feature = "server")]
impl PlayerGameStateView {
    #[must_use]
    pub fn new(
        player_states: Vec<PlayerPublicState>,
        phase: GamePhaseView,
        hand: Vec<Card>,
    ) -> Self {
        Self {
            player_states,
            phase,
            hand,
        }
    }
}
