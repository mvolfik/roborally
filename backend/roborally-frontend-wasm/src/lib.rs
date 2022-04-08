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
#![feature(let_chains)]
mod asset;
mod utils;

use crate::asset::AssetMap;
use roborally_structs::{
    animations::Animation,
    card::wrapper::CardWrapper,
    game_map::GameMap,
    game_state::{GamePhaseView, PlayerGameStateView, PlayerPublicState},
    logging::{self, debug, error},
    position::Direction,
    transport::{ClientMessage, ServerMessage},
};

use js_sys::Function;
use std::{convert::Into, iter::repeat_with, panic};
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

    // due to some weird bug in wasm-bindgen, compilation fails with longer argument names here
    #[wasm_bindgen(typescript_type = "(f: Position, t: Position, d: number, x: boolean) => void")]
    /// somehow enum.into::<JsValue>() isn't supported, so we use a flag 0..3=[Up Right Down Left] for direction
    pub type ProcessBulletClosure;
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
            _ => Err("Unexpected error when initializing connection".into()),
        }
    }

    pub fn handle_message(
        bytes: &[u8],
        set_state: SetStateFunction,
        notify: NotifyFunction,
        process_bullet_closure: ProcessBulletClosure,
    ) -> Result<(), JsValue> {
        match rmp_serde::from_slice::<ServerMessage>(bytes)
            .map_err::<JsValue, _>(|e| e.to_string().into())?
        {
            ServerMessage::Notice(msg) => notify.call(msg)?,
            ServerMessage::SetState(state) => {
                debug!("{:?}", &state);
                set_state.call(state)?;
            }
            ServerMessage::Animations(animations) => {
                let process_bullet = process_bullet_closure.unchecked_into::<js_sys::Function>();
                for animation in animations {
                    match animation {
                        Animation::BulletFlight(from, to, direction, is_from_tank) => {
                            let args: [JsValue; 4] = [
                                from.into(),
                                to.into(),
                                match direction {
                                    Direction::Up => 0_u8,
                                    Direction::Right => 1,
                                    Direction::Down => 2,
                                    Direction::Left => 3,
                                }
                                .into(),
                                is_from_tank.into(),
                            ];
                            if let Err(e) = process_bullet
                                .apply(&JsValue::UNDEFINED, &args.into_iter().collect())
                            {
                                error!("Error calling process_animation_closure: {:?}", e);
                            }
                        }
                    }
                }
            }
            ServerMessage::InitInfo(_) => {
                notify.call("Error: unexpected message from server".to_owned())?;
            }
        }
        Ok(())
    }

    #[must_use]
    pub fn create_init_message(name: String, seat: usize) -> Vec<u8> {
        rmp_serde::to_vec(&ClientMessage::Init { name, seat }).unwrap()
    }

    #[must_use]
    pub fn create_program_cards_message(
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
    pub fn get_artificial_spawn_state(&self) -> PlayerGameStateView {
        PlayerGameStateView::new(
            self.0
                .spawn_points
                .iter()
                .map(|(pos, dir)| PlayerPublicState {
                    position: *pos,
                    direction: dir.to_continuous(),
                    checkpoint: 0,
                    is_rebooting: false,
                })
                .collect(),
            GamePhaseView::HasWinner(0),
            Vec::new(),
            repeat_with(|| Some("Spawnpoint".to_owned()))
                .take(self.0.spawn_points.len())
                .collect(),
        )
    }
}
