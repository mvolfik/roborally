use crate::{
    card::Card,
    game_state::{animated_state::AnimationItem, GeneralState, ProgrammingState},
};

use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "server", derive(Serialize))]
#[cfg_attr(feature = "client", derive(Deserialize))]
#[derive(Debug, Clone)]
pub enum ServerMessage {
    Notice(String),
    GameLog(String),
    GeneralState(GeneralState),
    ProgrammingState(ProgrammingState),
    AnimatedState(AnimationItem),
}

#[cfg(feature = "client")]
pub mod wrapper {
    use wasm_bindgen::prelude::wasm_bindgen;

    use crate::game_state::{animated_state::AnimationItem, GeneralState, ProgrammingState};

    use super::ServerMessage;

    #[wasm_bindgen]
    pub enum ServerMessageType {
        Notice,
        GameLog,
        GeneralState,
        ProgrammingState,
        AnimatedState,
    }

    #[wasm_bindgen(skip_all)]
    pub struct ServerMessageWrapper(pub ServerMessage);

    #[wasm_bindgen]
    impl ServerMessageWrapper {
        #[wasm_bindgen(getter)]
        #[must_use]
        pub fn typ(&self) -> ServerMessageType {
            match &self.0 {
                ServerMessage::Notice(_) => ServerMessageType::Notice,
                ServerMessage::GameLog(_) => ServerMessageType::GameLog,
                ServerMessage::GeneralState(_) => ServerMessageType::GeneralState,
                ServerMessage::ProgrammingState(_) => ServerMessageType::ProgrammingState,
                ServerMessage::AnimatedState(_) => ServerMessageType::AnimatedState,
            }
        }

        #[wasm_bindgen(getter)]
        #[must_use]
        pub fn notice(&self) -> String {
            if let ServerMessage::Notice(s) = &self.0 {
                s.clone()
            } else {
                panic!("Tried to get notice from different message type");
            }
        }

        #[wasm_bindgen(getter)]
        #[must_use]
        pub fn game_log(&self) -> String {
            if let ServerMessage::GameLog(s) = &self.0 {
                s.clone()
            } else {
                panic!("Tried to get game_log from different message type");
            }
        }

        #[wasm_bindgen(getter)]
        #[must_use]
        pub fn general_state(&self) -> GeneralState {
            if let ServerMessage::GeneralState(s) = &self.0 {
                s.clone()
            } else {
                panic!("Tried to get general_state from different message type");
            }
        }

        #[wasm_bindgen(getter)]
        #[must_use]
        pub fn programming_state(&self) -> ProgrammingState {
            if let ServerMessage::ProgrammingState(s) = &self.0 {
                s.clone()
            } else {
                panic!("Tried to get programming_state from different message type");
            }
        }

        #[wasm_bindgen(getter)]
        #[must_use]
        pub fn animated_state(&self) -> AnimationItem {
            if let ServerMessage::AnimatedState(s) = &self.0 {
                s.clone()
            } else {
                panic!("Tried to get animated_state from different message type");
            }
        }
    }
}

#[cfg_attr(feature = "server", derive(Deserialize, Debug))]
#[cfg_attr(feature = "client", derive(Serialize))]
pub enum ClientMessage {
    Program(Vec<Card>),
}
