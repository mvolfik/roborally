#![warn(clippy::nursery)]
#![allow(clippy::use_self)]
#![allow(clippy::future_not_send)]
#![warn(clippy::pedantic)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::unused_unit)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::enum_glob_use)]
#![allow(clippy::many_single_char_names)]
#![warn(clippy::shadow_reuse)]
#![warn(clippy::shadow_same)]
#![warn(clippy::shadow_unrelated)]
#![feature(pattern)]
#![feature(const_precise_live_drops)]
mod asset;
mod utils;

use crate::asset::AssetMap;
use roborally_structs::{
    card::wrapper::CardWrapper,
    game_map::GameMap,
    game_state::PlayerGameStateView,
    logging::{self, debug},
    transport::{ClientMessage, ServerMessage},
};

use js_sys::Function;
use std::panic;
use wasm_bindgen::{prelude::*, JsCast};

/* ##### INIT ##### */
#[wasm_bindgen(start)]
pub fn run_initializations() {
    panic::set_hook(Box::new(console_error_panic_hook::hook));
    logging::init();
}
/* ##### /INIT ##### */

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "(val: PlayerGameStateView) => void")]
    pub type SetStateFunction;

    #[wasm_bindgen(typescript_type = "(msg: string) => void")]
    pub type NotifyFunction;
}

impl SetStateFunction {
    fn call(self, state: PlayerGameStateView) -> Result<(), JsValue> {
        self.unchecked_into::<Function>()
            .call1(&JsValue::UNDEFINED, &state.into())?;
        Ok(())
    }
}

impl NotifyFunction {
    fn call(self, msg: String) -> Result<(), JsValue> {
        self.unchecked_into::<Function>()
            .call1(&JsValue::UNDEFINED, &msg.into())?;
        Ok(())
    }
}

#[wasm_bindgen]
pub struct MessageProcessor;

#[wasm_bindgen]
impl MessageProcessor {
    pub fn expect_init_message(bytes: &[u8]) -> Result<AssetMap, JsValue> {
        match rmp_serde::from_slice::<ServerMessage>(bytes)
            .map_err::<JsValue, _>(|e| e.to_string().into())?
        {
            ServerMessage::Notice(msg) => Err(msg.into()),
            ServerMessage::InitInfo(map) => Ok(map.into()),
            ServerMessage::SetState(_) => Err("Unexpected error when initializing connection".into()),
        }
    }

    pub fn handle_message(
        bytes: &[u8],
        set_state: SetStateFunction,
        notify: NotifyFunction,
    ) -> Result<(), JsValue> {
        match rmp_serde::from_slice::<ServerMessage>(bytes)
            .map_err::<JsValue, _>(|e| e.to_string().into())?
        {
            ServerMessage::Notice(msg) => notify.call(msg)?,
            ServerMessage::SetState(state) => {
                debug!("{:?}", &state);
                set_state.call(state)?;
            }
            ServerMessage::InitInfo(_) => notify.call("Error: unexpected message from server".to_string())?,
        }
        Ok(())
    }

    #[must_use] pub fn create_init_message(name: String, seat: usize) -> Vec<u8> {
        rmp_serde::to_vec(&ClientMessage::Init { name, seat }).unwrap()
    }

    #[must_use] pub fn create_program_cards_message(
        card1: &CardWrapper,
        card2: &CardWrapper,
        card3: &CardWrapper,
        card4: &CardWrapper,
        card5: &CardWrapper,
    ) -> Vec<u8> {
        rmp_serde::to_vec(&ClientMessage::Program([
            **card1, **card2, **card3, **card4, **card5,
        ]))
        .unwrap()
    }
}

#[wasm_bindgen]
pub fn parse_map(bytes: &[u8]) -> Result<AssetMap, JsValue> {
    rmp_serde::from_slice::<GameMap>(bytes)
        .map(std::convert::Into::into)
        .map_err(|e| e.to_string().into())
}
