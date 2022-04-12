use crate::{
    animations::Animation, card::Card, game_state::PlayerGameStateView, position::Direction,
};

use serde::{Deserialize, Serialize};

#[cfg(feature = "client")]
use wasm_bindgen::{prelude::wasm_bindgen, JsCast, JsValue};

#[cfg_attr(feature = "server", derive(Serialize))]
#[cfg_attr(feature = "client", derive(Deserialize), wasm_bindgen)]
#[derive(Debug, Clone)]
pub struct StateArrayItem {
    state: Option<PlayerGameStateView>,
    animations: Vec<Animation>,
}

#[cfg(feature = "server")]
impl StateArrayItem {
    pub fn new(state: Option<PlayerGameStateView>, animations: Vec<Animation>) -> Self {
        Self { state, animations }
    }
}

#[cfg(feature = "client")]
#[wasm_bindgen]
extern "C" {
    // due to some weird bug in wasm-bindgen, compilation fails with longer argument names here
    #[wasm_bindgen(typescript_type = "(f: Position, t: Position, d: number, x: boolean) => void")]
    /// somehow enum.into::<JsValue>() isn't supported, so we use a flag 0..3=[Up Right Down Left] for direction
    pub type ProcessBulletClosure;
}

#[cfg(feature = "client")]
#[wasm_bindgen]
impl StateArrayItem {
    #[wasm_bindgen(getter)]
    pub fn state(&self) -> Option<PlayerGameStateView> {
        self.state.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn has_state(&self) -> bool {
        self.state.is_some()
    }

    pub fn process_animations(
        &self,
        process_bullet_closure: ProcessBulletClosure,
    ) -> Result<(), JsValue> {
        let process_bullet_jsfunc = process_bullet_closure.unchecked_into::<js_sys::Function>();
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
                    process_bullet_jsfunc.apply(
                        &JsValue::UNDEFINED,
                        &args.into_iter().map(JsValue::from).collect(),
                    )?;
                }
            };
        }
        Ok(())
    }
}

#[cfg_attr(feature = "server", derive(Serialize))]
#[cfg_attr(feature = "client", derive(Deserialize))]
#[derive(Debug, Clone)]
pub enum ServerMessage {
    State(PlayerGameStateView),
    Notice(String),
    AnimatedStates(Vec<StateArrayItem>),
}

#[cfg_attr(feature = "server", derive(Deserialize, Debug))]
#[cfg_attr(feature = "client", derive(Serialize))]
pub enum ClientMessage {
    Init { name: String, seat: usize },
    Program([Card; 5]),
}
