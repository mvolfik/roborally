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
                SPAM => intern("spam.png").to_owned(),
                Worm => intern("worm.png").to_owned(),
                Virus => intern("virus.png").to_owned(),
                Trojan => intern("trojan.png").to_owned(),
                Move1 => intern("move1.png").to_owned(),
                Move2 => intern("move2.png").to_owned(),
                Move3 => intern("move3.png").to_owned(),
                Reverse1 => intern("reverse1.png").to_owned(),
                TurnRight => intern("turn-right.png").to_owned(),
                TurnLeft => intern("turn-left.png").to_owned(),
                UTurn => intern("u-turn.png").to_owned(),
                Again => intern("again.png").to_owned(),
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
