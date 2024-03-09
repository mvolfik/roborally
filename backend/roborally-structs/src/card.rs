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
