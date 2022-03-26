use crate::{game_map::GameMap, game_state::PlayerGameStateView, Card};

use serde::{Deserialize, Serialize};

#[cfg(feature = "client")]
use wasm_bindgen::prelude::wasm_bindgen;
use web_sys::UrlSearchParams;

#[cfg_attr(feature = "server", derive(Deserialize))]
#[cfg_attr(feature = "client", wasm_bindgen(inspectable))]
pub struct ConnectInfo {
    pub game_id: u64,
    pub player_i: usize,
    name: String,
}

#[cfg(feature = "client")]
#[wasm_bindgen]
impl ConnectInfo {
    #[wasm_bindgen(constructor)]
    #[must_use]
    pub fn new(game_id: u64, player_i: usize, name: String) -> Self {
        Self {
            game_id,
            player_i,
            name,
        }
    }
    #[wasm_bindgen(getter)]
    pub fn name(&self) -> String {
        self.name.clone()
    }
}
impl ConnectInfo {
    #[cfg(feature = "client")]
    pub fn to_query_string(&self) -> UrlSearchParams {
        let params = UrlSearchParams::new().unwrap();
        params.append("game_id", &self.game_id.to_string());
        params.append("player_i", &self.player_i.to_string());
        params.append("name", &self.name);
        params
    }

    #[cfg(not(feature = "client"))]
    pub fn name(&self) -> &str {
        &self.name
    }
}

#[cfg_attr(feature = "server", derive(Serialize))]
#[cfg_attr(feature = "client", derive(Deserialize))]
#[derive(Debug)]
pub enum ServerMessage {
    InitInfo {
        map: GameMap,
        state: PlayerGameStateView,
    },
    SetState(PlayerGameStateView),
}

#[cfg_attr(feature = "server", derive(Deserialize))]
#[cfg_attr(feature = "client", derive(Serialize))]
pub enum ClientMessage {
    Program([Card; 5]),
}
