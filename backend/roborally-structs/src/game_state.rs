use crate::{card::Card, position::Direction, position::Position};
use serde::{Deserialize, Serialize};

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
    RobotLasers,
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
    },
    Programming {
        ready: Vec<bool>,
        my_cards: Option<[Card; 5]>,
    },
    HasWinner(usize),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "server", derive(Serialize))]
#[cfg_attr(feature = "client", derive(Deserialize))]
pub struct PlayerPublicState {
    pub position: Position,
    pub direction: Direction,
    pub checkpoint: usize,
    pub is_rebooting: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "server", derive(Serialize))]
#[cfg_attr(feature = "client", derive(Deserialize), wasm_bindgen)]
pub struct PlayerGameStateView {
    player_states: Vec<PlayerPublicState>,
    phase: GamePhaseView,
    hand: Vec<Card>,
    player_names: Vec<Option<String>>,
}

#[cfg(feature = "server")]
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
    pub fn hand_len(&self) -> usize {
        self.hand.len()
    }
    pub fn get_hand_card(&self, i: usize) -> Option<crate::card::wrapper::CardWrapper> {
        Some(crate::card::wrapper::CardWrapper(*self.hand.get(i)?))
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
    #[wasm_bindgen(getter)]
    pub fn players(&self) -> usize {
        self.player_states.len()
    }
    pub fn get_player(&self, i: usize) -> Option<wrapper::PlayerPublicStateWrapper> {
        if let (Some(name), Some(player)) = (self.player_names.get(i), self.player_states.get(i)) {
            Some(wrapper::PlayerPublicStateWrapper(
                *player,
                name.as_ref().map(|x| x.clone()),
                i,
            ))
        } else {
            None
        }
    }

    pub fn is_ready_programming(&self, i: usize) -> Option<bool> {
        if let GamePhaseView::Programming { ready, .. } = &self.phase {
            ready.get(i).copied()
        } else {
            None
        }
    }
}

#[cfg(feature = "client")]
mod wrapper {
    use wasm_bindgen::prelude::wasm_bindgen;

    use crate::{position::Position, transform::Transform};

    use super::PlayerPublicState;

    #[wasm_bindgen]
    pub struct PlayerPublicStateWrapper(
        pub(super) PlayerPublicState,
        pub(super) Option<String>,
        pub(super) usize,
    );

    #[wasm_bindgen]
    impl PlayerPublicStateWrapper {
        #[wasm_bindgen(getter)]
        pub fn position(&self) -> Position {
            self.0.position
        }

        #[wasm_bindgen(getter)]
        /// Note: doesn't include transform to current tile
        pub fn transform_string(&self) -> String {
            Transform {
                rotate: self.0.direction.get_rotation(),
                ..Transform::default()
            }
            .to_string()
        }

        #[wasm_bindgen(getter)]
        /// Note: doesn't include transform to current tile
        pub fn filter_string(&self) -> String {
            format!("hue-rotate({}rad)", self.2 as f64 * 0.9)
        }

        #[wasm_bindgen(getter)]
        pub fn name(&self) -> Option<String> {
            self.1.as_ref().map(|x| x.clone())
        }
    }
}
