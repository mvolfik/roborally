use crate::create_array_type;
use crate::position::ContinuousDirection;
use crate::position::Position;
use crate::transform::Effects;
use serde::{Deserialize, Serialize};

#[cfg(feature = "client")]
use wasm_bindgen::prelude::wasm_bindgen;

#[derive(Clone, Debug, Deserialize)]
#[cfg_attr(feature = "server", derive(Serialize))]
#[cfg_attr(feature = "client", wasm_bindgen(skip_all))]
#[allow(clippy::unsafe_derive_deserialize)]
/// Public state of 1 player
pub struct PlayerPublicState {
    pub position: Position,
    pub direction: ContinuousDirection,
    pub checkpoint: usize,
    pub is_rebooting: bool,
    /// purely presentational: used during reboot
    pub is_hidden: bool,
}

#[cfg(feature = "client")]
#[wasm_bindgen]
impl PlayerPublicState {
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn position(&self) -> Position {
        self.position
    }

    #[wasm_bindgen(getter)]
    #[must_use]
    /// Note: doesn't include transform to current tile
    pub fn style(&self) -> String {
        Effects {
            rotate: self.direction,
            scale: 0.125,
            ..Effects::default()
        }
        .to_string()
    }
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn checkpoint(&self) -> usize {
        self.checkpoint
    }

    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn is_rebooting(&self) -> bool {
        self.is_rebooting
    }

    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn is_hidden(&self) -> bool {
        self.is_hidden
    }
}

#[cfg(feature = "client")]
create_array_type!(name: PlayerPublicStateArray, full_js_type: "Array<PlayerPublicState>", rust_inner_type: PlayerPublicState);
