use crate::{animations::Animation, card::Card, create_array_type, position::Direction};
use serde::{Deserialize, Serialize};

#[cfg(feature = "client")]
use wasm_bindgen::{
    prelude::wasm_bindgen,
    {JsCast, JsValue},
};

use super::{phase::RegisterMovePhase, player_public_state::PlayerPublicState};

#[derive(Clone, Debug)]
#[cfg_attr(feature = "server", derive(Serialize))]
#[cfg_attr(feature = "client", derive(Deserialize), wasm_bindgen(skip_all))]
#[allow(clippy::unsafe_derive_deserialize)]
/// Player's view of the game - doesn't inlude other players' cards etc.
pub struct RunningStateView {
    pub register: usize,

    pub register_phase: RegisterMovePhase,

    pub my_cards: Vec<Card>,

    pub players_revealed_cards: Vec<Vec<Card>>,

    pub player_states: Vec<PlayerPublicState>,
}

#[cfg(feature = "client")]
#[wasm_bindgen]
extern "C" {
    // due to some weird bug in wasm-bindgen, compilation fails with longer argument names here
    #[wasm_bindgen(typescript_type = "(f: Position, t: Position, d: number, x: boolean) => void")]
    /// somehow enum.into::<JsValue>() isn't supported, so we use a flag 0..3=[Up Right Down Left] for direction
    pub type ProcessBulletClosure;

    #[wasm_bindgen(typescript_type = "(player_i: number) => void")]
    pub type ProcessCheckpointVisitedClosure;

    #[wasm_bindgen(typescript_type = "(player_i: number) => void")]
    pub type ClosurePlayerI;
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "server", derive(Serialize))]
#[cfg_attr(feature = "client", derive(Deserialize), wasm_bindgen(skip_all))]
#[allow(clippy::unsafe_derive_deserialize)]
pub struct AnimationItem {
    pub animations: Vec<Animation>,
    /// If None, the state didn't change, only some animations should play. Skip this state when iterating backwards
    pub state: Option<RunningStateView>,
}

#[cfg(feature = "client")]
create_array_type!(name: AnimationItemArray, full_js_type: "Array<AnimationItem>", rust_inner_type: AnimationItem);

#[cfg(feature = "client")]
#[wasm_bindgen]
impl AnimationItem {
    pub fn process_animations(
        &self,
        process_bullet_closure: ProcessBulletClosure,
        process_checkpoint_visited_closure: ProcessCheckpointVisitedClosure,
    ) -> Result<(), JsValue> {
        let process_bullet_jsfunc = process_bullet_closure.unchecked_into::<js_sys::Function>();
        let process_checkpoint_visited_jsfunc =
            process_checkpoint_visited_closure.unchecked_into::<js_sys::Function>();
        for animation in &self.animations {
            match animation {
                Animation::BulletFlight {
                    from,
                    to,
                    direction,
                    is_from_tank,
                } => {
                    let args: [JsValue; 4] = [
                        (*from).into(),
                        (*to).into(),
                        match direction {
                            Direction::Up => 0_u8,
                            Direction::Right => 1,
                            Direction::Down => 2,
                            Direction::Left => 3,
                        }
                        .into(),
                        (*is_from_tank).into(),
                    ];
                    process_bullet_jsfunc
                        .apply(&JsValue::UNDEFINED, &args.into_iter().collect())?;
                }
                Animation::CheckpointVisited { player_i } => {
                    process_checkpoint_visited_jsfunc
                        .call1(&JsValue::UNDEFINED, &(*player_i as u8).into())?;
                }
            };
        }
        Ok(())
    }

    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn has_state(&self) -> bool {
        self.state.is_some()
    }

    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn register(&self) -> usize {
        self.state.as_ref().unwrap().register
    }

    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn register_phase(&self) -> RegisterMovePhase {
        self.state.as_ref().unwrap().register_phase
    }

    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn my_cards(&self) -> Vec<u8> {
        self.state
            .as_ref()
            .unwrap()
            .my_cards
            .iter()
            .map(|c| c.to_number())
            .collect()
    }

    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn player_states(&self) -> super::player_public_state::PlayerPublicStateArray {
        self.state
            .as_ref()
            .unwrap()
            .player_states
            .clone()
            .into_iter()
            .collect()
    }

    #[must_use]
    pub fn get_revealed_cards(&self, player_i: usize) -> Vec<u8> {
        self.state.as_ref().unwrap().players_revealed_cards[player_i]
            .iter()
            .map(|c| c.to_number())
            .collect()
    }
}
