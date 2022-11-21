pub mod animated_state;
pub mod phase;
pub mod player_public_state;

use crate::card::Card;
use serde::{Deserialize, Serialize};

#[cfg(feature = "client")]
use wasm_bindgen::prelude::wasm_bindgen;

use self::player_public_state::PlayerPublicState;

#[derive(Clone, Debug)]
#[cfg_attr(feature = "server", derive(Serialize))]
#[cfg_attr(feature = "client", derive(Deserialize))]
pub enum GameStatusInfo {
    Programming,
    Processing,
}

impl std::fmt::Display for GameStatusInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GameStatusInfo::Programming => write!(f, "Waiting for players to program their robots"),
            GameStatusInfo::Processing => write!(f, "Evaluating moves"),
        }
    }
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "server", derive(Serialize))]
#[cfg_attr(feature = "client", derive(Deserialize), wasm_bindgen(skip_all))]
#[allow(clippy::unsafe_derive_deserialize)]
/// Player's view of the game - doesn't inlude other players' cards etc.
pub struct GeneralState {
    pub player_names: Vec<Option<String>>,
    pub status: GameStatusInfo,
}

#[cfg(feature = "client")]
#[wasm_bindgen]
impl GeneralState {
    #[must_use]
    pub fn get_player_name(&self, player_i: usize) -> Option<String> {
        self.player_names[player_i].clone()
    }

    #[must_use]
    #[wasm_bindgen(getter)]
    pub fn status(&self) -> String {
        self.status.to_string()
    }
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "server", derive(Serialize))]
#[cfg_attr(feature = "client", derive(Deserialize), wasm_bindgen(skip_all))]
#[allow(clippy::unsafe_derive_deserialize)]
pub struct ProgrammingState {
    pub hand: Vec<Card>,
    pub prepared_cards: Option<Vec<Card>>,
    pub ready_players: Vec<bool>,
    pub player_states: Vec<PlayerPublicState>,
}

#[cfg(feature = "client")]
#[wasm_bindgen]
impl ProgrammingState {
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn hand(&self) -> Vec<u8> {
        self.hand.iter().map(|c| c.to_number()).collect()
    }

    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn prepared_cards(&self) -> Option<Vec<u8>> {
        self.prepared_cards
            .as_ref()
            .map(|v| v.iter().map(|c| c.to_number()).collect())
    }

    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn ready_players(&self) -> Vec<u8> {
        self.ready_players.iter().map(|b| u8::from(*b)).collect()
    }

    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn player_states(&self) -> self::player_public_state::PlayerPublicStateArray {
        self.player_states.clone().into_iter().collect()
    }
}
