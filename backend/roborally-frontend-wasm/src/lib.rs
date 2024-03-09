#![warn(clippy::nursery)]
#![allow(clippy::use_self)]
#![allow(clippy::missing_const_for_fn)]
#![warn(clippy::pedantic)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_possible_wrap)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::unused_unit)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::enum_glob_use)]
#![allow(clippy::many_single_char_names)]
// restrictions
#![warn(clippy::allow_attributes_without_reason)]
#![warn(clippy::clone_on_ref_ptr)]
#![warn(clippy::else_if_without_else)]
#![warn(clippy::get_unwrap)]
#![warn(clippy::if_then_some_else_none)]
#![warn(clippy::let_underscore_must_use)]
#![warn(clippy::shadow_reuse)]
#![warn(clippy::shadow_same)]
#![warn(clippy::shadow_unrelated)]
#![warn(clippy::str_to_string)]
#![warn(clippy::string_add)]
#![warn(clippy::string_to_string)]
#![warn(clippy::try_err)]
// features
#![feature(pattern)]
#![feature(const_precise_live_drops)]
#![feature(let_chains)]
mod asset;

use crate::asset::AssetMap;
use roborally_structs::{
    card::Card,
    game_map::GameMap,
    game_state::player_public_state::{PlayerPublicState, PlayerPublicStateArray},
    logging::{self, info},
    transport::{wrapper::ServerMessageWrapper, ClientMessage, ServerMessage},
};

use wasm_bindgen::prelude::{wasm_bindgen, JsValue};

/* ##### INIT ##### */
#[wasm_bindgen(start)]
pub fn run_initializations() {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));
    logging::init();
    info!("WASM module initialized");
}
/* ##### /INIT ##### */

#[wasm_bindgen]
pub fn parse_message(bytes: &[u8]) -> Result<ServerMessageWrapper, JsValue> {
    rmp_serde::from_slice::<ServerMessage>(bytes)
        .map(ServerMessageWrapper)
        .map_err::<JsValue, _>(|e| e.to_string().into())
}

#[wasm_bindgen]
#[must_use]
pub fn create_program_cards_message(cards: Vec<u8>) -> Vec<u8> {
    rmp_serde::to_vec(&ClientMessage::Program(
        cards.into_iter().map(Card::from_number).collect(),
    ))
    .unwrap()
}

#[wasm_bindgen]
pub fn parse_map(bytes: &[u8]) -> Result<ParsedMap, JsValue> {
    rmp_serde::from_slice::<GameMap>(bytes)
        .map(ParsedMap)
        .map_err(|e| e.to_string().into())
}

#[wasm_bindgen]
pub struct ParsedMap(GameMap);

#[wasm_bindgen]
impl ParsedMap {
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn assets(&self) -> AssetMap {
        self.0.clone().into()
    }

    #[must_use]
    /// Used to render a map preview
    ///
    /// A specific asset for "Spawnpoint" doesn't exist, so for map preview, this method creates an artificial
    /// state with a robot named "Spawnpoint" at each spawnpoint location
    pub fn get_artificial_spawn_state(&self) -> PlayerPublicStateArray {
        self.0
            .spawn_points
            .iter()
            .map(|(pos, dir)| PlayerPublicState {
                position: *pos,
                direction: dir.to_continuous(),
                checkpoint: 0,
                is_rebooting: false,
                is_hidden: false,
            })
            .collect()
    }
}
