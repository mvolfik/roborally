use crate::{
    animations::Animation, card::Card, game_map::GameMap, game_state::PlayerGameStateView,
};

use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "server", derive(Serialize))]
#[cfg_attr(feature = "client", derive(Deserialize))]
#[derive(Debug)]
pub enum ServerMessage {
    InitInfo(GameMap),
    SetState(PlayerGameStateView),
    Notice(String),
    Animations(Vec<Animation>),
}

#[cfg_attr(feature = "server", derive(Deserialize, Debug))]
#[cfg_attr(feature = "client", derive(Serialize))]
pub enum ClientMessage {
    Init { name: String, seat: usize },
    Program([Card; 5]),
}
