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
    pub struct CardWrapper(#[wasm_bindgen(skip)] pub Card);

    #[wasm_bindgen]
    impl CardWrapper {
        #[wasm_bindgen(getter)]
        #[must_use]
        pub fn asset_name(&self) -> String {
            use Card::*;
            match self.0 {
                SPAM => intern("spam.png").to_string(),
                Worm => intern("worm.png").to_string(),
                Virus => intern("virus.png").to_string(),
                Trojan => intern("trojan.png").to_string(),
                Move1 => intern("move1.png").to_string(),
                Move2 => intern("move2.png").to_string(),
                Move3 => intern("move3.png").to_string(),
                Reverse1 => intern("reverse1.png").to_string(),
                TurnRight => intern("turn-right.png").to_string(),
                TurnLeft => intern("turn-left.png").to_string(),
                UTurn => intern("u-turn.png").to_string(),
                Again => intern("again.png").to_string(),
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
