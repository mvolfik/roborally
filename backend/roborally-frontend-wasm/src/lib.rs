#![warn(clippy::nursery)]
#![allow(clippy::use_self)]
#![allow(clippy::future_not_send)]
#![warn(clippy::pedantic)]
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
mod store;
mod utils;

use crate::{asset::AssetMap, store::Writable};
use futures::channel::oneshot::{channel, Receiver};
use roborally_structs::{
    transport::{ClientMessage, ConnectInfo, ServerMessage},
    wrapper::CardWrapper,
};

use js_sys::{ArrayBuffer, Function, Uint8Array};
use std::panic;
use utils::await_event;
use wasm_bindgen::{prelude::*, JsCast};
use web_sys::{console, window, MessageEvent, WebSocket};

///// INIT /////
#[wasm_bindgen(start)]
pub fn init_panic_hook() {
    panic::set_hook(Box::new(console_error_panic_hook::hook));
}
///// /INIT /////

enum ConnectionState {
    WaitingForInitData,
    InGame,
}

fn handle_error(e: impl JsCast, conn: &WebSocket) {
    console::error_1(&e.unchecked_into());
    if let Err(closing_error) = conn.close() {
        console::error_2(
            &"Additional error encountered when attempted to close connection".into(),
            &closing_error,
        );
    };
}

fn create_ws_message_handler(connection: WebSocket) -> (Function, Receiver<GameConnectionResult>) {
    let mut connection_state = ConnectionState::WaitingForInitData;
    let mut store = Writable::new(JsValue::UNDEFINED);
    let (resolve, receiver) = channel();
    let mut resolve_opt = Some(resolve);
    let closure = move |event: MessageEvent| {
        let buffer: ArrayBuffer = match event.data().dyn_into() {
            Ok(v) => v,
            Err(e) => {
                handle_error(e, &connection);
                return;
            }
        };
        let message: ServerMessage =
            match rmp_serde::from_slice(&Uint8Array::new(&buffer.unchecked_into()).to_vec()) {
                Ok(v) => v,
                Err(e) => {
                    handle_error(Into::<JsValue>::into(e.to_string()), &connection);
                    return;
                }
            };
        console::log_1(&format!("{:?}", message).into());
        let new_state = match (&connection_state, message) {
            (ConnectionState::WaitingForInitData, ServerMessage::InitInfo { map, state }) => {
                connection_state = ConnectionState::InGame;
                if let Some(res) = resolve_opt.take() {
                    if res
                        .send(GameConnectionResult {
                            store: Writable::clone(&store),
                            assets_map: map.into(),
                            connection: connection.clone(),
                        })
                        .is_err()
                    {
                        handle_error(
                            Into::<JsValue>::into(
                                "Receiver cancelled before connection was established",
                            ),
                            &connection,
                        );
                        return;
                    }
                }
                state
            }
            (ConnectionState::InGame, ServerMessage::SetState(state)) => state,
            _ => {
                handle_error(
                    Into::<JsValue>::into("Server and client state got out of sync"),
                    &connection,
                );
                return;
            }
        };
        store.set(new_state.into());
    };
    (
        Closure::<dyn FnMut(MessageEvent)>::new(closure)
            .into_js_value()
            .unchecked_into(),
        receiver,
    )
}

#[wasm_bindgen]
pub struct GameConnectionResult {
    store: Writable,
    assets_map: AssetMap,
    connection: WebSocket,
}

#[wasm_bindgen]
impl GameConnectionResult {
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn store(&self) -> Writable {
        self.store.clone()
    }
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn assets_map(&self) -> AssetMap {
        self.assets_map.clone()
    }
    pub fn program(
        &self,
        card1: &CardWrapper,
        card2: &CardWrapper,
        card3: &CardWrapper,
        card4: &CardWrapper,
        card5: &CardWrapper,
    ) -> Result<(), JsValue> {
        self.connection.send_with_u8_array(
            &rmp_serde::to_vec(&ClientMessage::Program([
                **card1, **card2, **card3, **card4, **card5,
            ]))
            .map_err(|e| JsValue::from(e.to_string()))?,
        )
    }
}

#[wasm_bindgen]
pub async fn connect_to_game(connect_info: ConnectInfo) -> Result<GameConnectionResult, JsValue> {
    let loc = window().unwrap().location();
    let ws = WebSocket::new(&format!(
        "ws{}://{}/game?{}",
        if loc.protocol()? == "https" { "s" } else { "" },
        loc.host()?,
        connect_info.to_query_string().to_string()
    ))?;
    ws.set_binary_type(web_sys::BinaryType::Arraybuffer);
    let (handler, init_future) = create_ws_message_handler(ws.clone());
    ws.add_event_listener_with_callback("message", &handler)?;

    await_event(&ws, "open")?
        .await
        .map_err(|_| "failed to establish connection")?;

    Ok(init_future
        .await
        .map_err(|_| "failed to establish connection")?)
}
