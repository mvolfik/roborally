use crate::{game_map::GameMap, game_state::PlayerGameStateView, card::Card};

use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "server", derive(Serialize))]
#[cfg_attr(feature = "client", derive(Deserialize))]
#[derive(Debug)]
pub enum ServerMessage {
    InitInfo {
        map: GameMap,
        state: PlayerGameStateView,
    },
    SetState(PlayerGameStateView),
    Notice(String),
}

#[cfg_attr(feature = "server", derive(Deserialize, Debug))]
#[cfg_attr(feature = "client", derive(Serialize))]
pub enum ClientMessage {
    Init { name: String, seat: usize },
    Program([Card; 5]),
}
