use serde::{Deserialize, Serialize};

#[cfg(feature = "client")]
use wasm_bindgen::prelude::wasm_bindgen;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "server", derive(Serialize))]
#[cfg_attr(feature = "client", derive(Deserialize), wasm_bindgen)]
pub enum RegisterMovePhase {
    PlayerCards,
    FastBelts,
    SlowBelts,
    PushPanels,
    Rotations,
    Lasers,
    Checkpoints,
}

impl RegisterMovePhase {
    pub const ORDER: [Self; 7] = [
        Self::PlayerCards,
        Self::FastBelts,
        Self::SlowBelts,
        Self::PushPanels,
        Self::Rotations,
        Self::Lasers,
        Self::Checkpoints,
    ];
}
