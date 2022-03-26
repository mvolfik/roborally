#![warn(clippy::nursery)]
#![allow(clippy::use_self)]
#![warn(clippy::pedantic)]
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

pub mod game_map;
pub mod game_state;
pub mod position;
pub mod tile;
pub mod tile_type;
pub mod transport;

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Card {
    SPAM,
    Worm,
    Virus,
    Trojan,
    Move1,
    Move2,
    Move3,
    Reverse1,
    TurnRight,
    TurnLeft,
    UTurn,
    Again,
}

#[cfg(feature = "client")]
pub mod wrapper {
    use std::ops::Deref;

    use super::Card;
    use wasm_bindgen::{intern, prelude::wasm_bindgen};

    #[wasm_bindgen]
    pub struct CardWrapper(Card);

    #[wasm_bindgen]
    impl CardWrapper {
        #[wasm_bindgen(getter)]
        #[must_use]
        pub fn asset_name(&self) -> String {
            match self.0 {
                Card::SPAM => intern("card/spam.png").to_string(),
                Card::Worm => intern("card/worm.png").to_string(),
                Card::Virus => intern("card/virus.png").to_string(),
                Card::Trojan => intern("card/trojan.png").to_string(),
                Card::Move1 => intern("card/move1.png").to_string(),
                Card::Move2 => intern("card/move2.png").to_string(),
                Card::Move3 => intern("card/move3.png").to_string(),
                Card::Reverse1 => intern("card/reverse1.png").to_string(),
                Card::TurnRight => intern("card/turn-right.png").to_string(),
                Card::TurnLeft => intern("card/turn-left.png").to_string(),
                Card::UTurn => intern("card/u-turn.png").to_string(),
                Card::Again => intern("card/again.png").to_string(),
            }
        }
    }

    impl Deref for CardWrapper {
        type Target = Card;

        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }
}
