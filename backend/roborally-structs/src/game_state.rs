use crate::{
    card::Card,
    position::{ContinuousDirection, Position},
};
use serde::{Deserialize, Serialize};

#[cfg(feature = "client")]
use crate::{
    card::wrapper::{CardWrapper, CardWrapperArray},
    game_state::wrapper::{PlayerPublicStateWrapper, PlayerPublicStateWrapperArray},
};

#[cfg(feature = "client")]
use wasm_bindgen::prelude::wasm_bindgen;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "server", derive(Serialize))]
#[cfg_attr(feature = "client", derive(Deserialize))]
pub enum RegisterMovePhase {
    PlayerCards,
    FastBelts,
    SlowBelts,
    PushPanels,
    Rotations,
    Lasers,
    Checkpoints,
}
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "server", derive(Serialize))]
#[cfg_attr(feature = "client", derive(Deserialize))]
pub enum GamePhaseView {
    Moving {
        register: usize,
        register_phase: RegisterMovePhase,
        // Only cards for this register are visible
        cards: Vec<Card>,
        my_registers: [Card; 5],
    },
    Programming {
        ready: Vec<bool>,
        my_cards: Option<[Card; 5]>,
    },
    HasWinner(usize),
}

#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "server", derive(Serialize))]
#[cfg_attr(feature = "client", derive(Deserialize))]
/// Public state of 1 player
pub struct PlayerPublicState {
    pub position: Position,
    pub direction: ContinuousDirection,
    pub checkpoint: usize,
    pub is_rebooting: bool,
    /// purely presentational: used during reboot
    pub is_hidden: bool,
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "server", derive(Serialize))]
#[cfg_attr(feature = "client", derive(Deserialize), wasm_bindgen)]
/// Player's view of the game - doesn't inlude other players' cards etc.
pub struct PlayerGameStateView {
    player_states: Vec<PlayerPublicState>,
    phase: GamePhaseView,
    hand: Vec<Card>,
    player_names: Vec<Option<String>>,
}

impl PlayerGameStateView {
    #[must_use]
    pub fn new(
        player_states: Vec<PlayerPublicState>,
        phase: GamePhaseView,
        hand: Vec<Card>,
        player_names: Vec<Option<String>>,
    ) -> Self {
        Self {
            player_states,
            phase,
            hand,
            player_names,
        }
    }
}

#[cfg(feature = "client")]
#[wasm_bindgen]
pub enum GamePhase {
    Programming,
    ProgrammingMyselfDone,
    Moving,
    HasWinner,
}

#[cfg(feature = "client")]
#[wasm_bindgen]
impl PlayerGameStateView {
    #[wasm_bindgen(getter)]
    pub fn hand(&self) -> CardWrapperArray {
        CardWrapperArray::from_iter(self.hand.iter().map(|c| CardWrapper(*c)))
    }
    #[wasm_bindgen(getter)]
    pub fn phase(&self) -> GamePhase {
        match self.phase {
            GamePhaseView::Programming { my_cards: None, .. } => GamePhase::Programming,
            GamePhaseView::Programming {
                my_cards: Some(_), ..
            } => GamePhase::ProgrammingMyselfDone,
            GamePhaseView::Moving { .. } => GamePhase::Moving,
            GamePhaseView::HasWinner(..) => GamePhase::HasWinner,
        }
    }

    #[wasm_bindgen]
    pub fn get_my_register_card(&self, i: usize) -> Option<CardWrapper> {
        match self.phase {
            GamePhaseView::Moving { my_registers, .. } => {
                my_registers.get(i).map(|c| CardWrapper(*c))
            }
            GamePhaseView::Programming { my_cards, .. } => my_cards
                .and_then(|cs| cs.into_iter().nth(i))
                .map(|c| CardWrapper(c)),
            GamePhaseView::HasWinner(_) => None,
        }
    }
    #[wasm_bindgen(getter)]
    pub fn moving_phase_register_number(&self) -> usize {
        if let GamePhaseView::Moving { register, .. } = self.phase {
            register
        } else {
            0
        }
    }
    #[wasm_bindgen(getter)]
    pub fn moving_phase_register_phase(&self) -> usize {
        if let GamePhaseView::Moving { register_phase, .. } = self.phase {
            register_phase as usize
        } else {
            0
        }
    }

    #[wasm_bindgen(getter)]
    pub fn players(&self) -> PlayerPublicStateWrapperArray {
        PlayerPublicStateWrapperArray::from_iter(
            self.player_states
                .iter()
                .zip(self.player_names.iter())
                .enumerate()
                .map(|(i, (state, name))| PlayerPublicStateWrapper {
                    state: *state,
                    name: name.clone(),
                    seat: i,
                }),
        )
    }

    pub fn is_ready_programming(&self, i: usize) -> Option<bool> {
        if let GamePhaseView::Programming { ready, .. } = &self.phase {
            ready.get(i).copied()
        } else {
            None
        }
    }

    pub fn get_player_card_for_current_register(&self, player_i: usize) -> Option<CardWrapper> {
        if let GamePhaseView::Moving { cards, .. } = &self.phase {
            cards.get(player_i).map(|c| CardWrapper(*c))
        } else {
            None
        }
    }

    pub fn get_winner_name(&self) -> Option<String> {
        if let GamePhaseView::HasWinner(player_i) = &self.phase {
            Some(match self.player_names.get(*player_i).cloned().flatten() {
                None => format!("Player {}", player_i),
                Some(name) => name,
            })
        } else {
            None
        }
    }
}

#[cfg(feature = "client")]
mod wrapper {
    use wasm_bindgen::prelude::wasm_bindgen;

    use crate::{create_array_type, position::Position, transform::Effects};

    use super::PlayerPublicState;

    #[wasm_bindgen]
    pub struct PlayerPublicStateWrapper {
        pub(super) state: PlayerPublicState,
        pub(super) name: Option<String>,
        pub(super) seat: usize,
    }

    create_array_type!( name: PlayerPublicStateWrapperArray, full_js_type: "Array<PlayerPublicStateWrapper>", rust_inner_type: PlayerPublicStateWrapper);

    #[wasm_bindgen]
    impl PlayerPublicStateWrapper {
        #[wasm_bindgen(getter)]
        pub fn position(&self) -> Position {
            self.state.position
        }

        #[wasm_bindgen(getter)]
        /// Note: doesn't include transform to current tile
        pub fn style(&self) -> String {
            Effects {
                rotate: self.state.direction,
                hue_shift: self.seat as f64 * 0.9,
                scale: 0.125,
                ..Effects::default()
            }
            .to_string()
        }

        #[wasm_bindgen(getter)]
        pub fn name(&self) -> Option<String> {
            self.name.clone()
        }

        #[wasm_bindgen(getter)]
        pub fn checkpoint(&self) -> usize {
            self.state.checkpoint
        }

        #[wasm_bindgen(getter)]
        pub fn is_rebooting(&self) -> bool {
            self.state.is_rebooting
        }

        #[wasm_bindgen(getter)]
        pub fn is_hidden(&self) -> bool {
            self.state.is_hidden
        }
    }
}
