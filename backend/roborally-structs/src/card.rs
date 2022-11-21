use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Card {
    Again,
    SPAM,
    Custom(usize),
}

impl Card {
    #[must_use]
    pub fn to_number(self) -> u8 {
        match self {
            Card::Again => 0,
            Card::SPAM => 1,
            Card::Custom(n) => n as u8 + 2,
        }
    }

    #[must_use]
    pub fn from_number(n: u8) -> Self {
        match n {
            0 => Card::Again,
            1 => Card::SPAM,
            i => Card::Custom(i as usize - 2),
        }
    }
}

// /// `wasm_bindgen` doesn't support methods on enum, so it needs to be wrapped
// #[cfg(feature = "client")]
// pub mod wrapper {
//     use crate::create_array_type;

//     use super::Card;
//     use std::ops::Deref;
//     use wasm_bindgen::prelude::wasm_bindgen;

//     #[wasm_bindgen(skip_all)]
//     pub struct CardWrapper(pub Card);

//     #[wasm_bindgen]
//     impl CardWrapper {
//         #[wasm_bindgen(getter)]
//         #[must_use]
//         pub fn asset_url_i(&self) -> usize {
//             use Card::*;
//             match &self.0 {
//                 SPAM => 0,
//                 Again => 1,
//                 Custom(i) => i + 2,
//             }
//         }
//     }

//     impl Deref for CardWrapper {
//         type Target = Card;

//         fn deref(&self) -> &Self::Target {
//             &self.0
//         }
//     }

//     create_array_type!( name: CardArray, full_js_type: "Array<CardWrapper>", rust_inner_type: CardWrapper);
// }
