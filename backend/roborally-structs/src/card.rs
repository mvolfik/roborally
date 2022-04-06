use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Card {
    SPAM,
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
            intern(match self.0 {
                SPAM => "spam.png",
                Move1 => "move1.png",
                Move2 => "move2.png",
                Move3 => "move3.png",
                Reverse1 => "reverse1.png",
                TurnRight => "turn-right.png",
                TurnLeft => "turn-left.png",
                UTurn => "u-turn.png",
                Again => "again.png",
            })
            .to_owned()
        }
    }

    impl Deref for CardWrapper {
        type Target = Card;

        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }
}
