use std::{iter::repeat, mem, sync::Weak};

use rand::{seq::SliceRandom, thread_rng};
use roborally_structs::{
    card::Card,
    game_state::player_public_state::PlayerPublicState,
    position::{Direction, Position},
};
use serde::{Deserialize, Serialize};

use crate::{game::CardInitializationDefinition, game_connection::PlayerConnection};

#[derive(Serialize, Deserialize, Clone)]
pub struct Player {
    pub public_state: PlayerPublicState,
    draw_pile: Vec<Card>,
    pub hand: Vec<Card>,
    pub discard_pile: Vec<Card>,
    #[serde(skip)]
    pub connected: Weak<PlayerConnection>,
    pub prepared_cards: Option<Vec<Card>>,
}

impl Player {
    pub fn new(
        spawn_point: (Position, Direction),
        again_count: usize,
        card_definitions: &[CardInitializationDefinition],
        draw_cards: usize,
    ) -> Self {
        let mut p = Self {
            public_state: PlayerPublicState {
                position: spawn_point.0,
                direction: spawn_point.1.to_continuous(),
                checkpoint: 0,
                is_rebooting: false,
                is_hidden: false,
            },
            draw_pile: Vec::new(),
            hand: Vec::new(),
            discard_pile: card_definitions
                .iter()
                .enumerate()
                .flat_map(|(i, definition)| repeat(Card::Custom(i)).take(definition.count))
                .chain(repeat(Card::Again).take(again_count))
                .collect(),
            connected: Weak::new(),
            prepared_cards: None,
        };
        p.hand = p.draw_n_cards(draw_cards);
        p
    }

    pub fn draw_one_card(&mut self) -> Card {
        if let Some(c) = self.draw_pile.pop() {
            c
        } else {
            self.draw_pile = mem::take(&mut self.discard_pile);
            self.draw_pile.shuffle(&mut thread_rng());
            self.draw_pile.pop().unwrap()
        }
    }

    pub fn draw_n_cards(&mut self, n: usize) -> Vec<Card> {
        (0..n).map(|_| self.draw_one_card()).collect()
    }

    pub fn draw_spam(&mut self) {
        self.discard_pile.push(Card::SPAM);
    }

    pub fn program(&mut self, cards: Vec<Card>) -> Result<(), String> {
        if self.prepared_cards.is_some() {
            return Err("Cards already set".to_owned());
        }

        let mut used_hand_indexes = vec![false; self.hand.len()];
        'outer: for picked_card in &cards {
            for (hand_card, used) in self.hand.iter().zip(used_hand_indexes.iter_mut()) {
                if hand_card == picked_card && !*used {
                    *used = true;
                    continue 'outer;
                }
            }
            // did not find this card (unused) in hand
            return Err(format!(
                "No cheating! {:?} isn't in your hand (enough times)",
                picked_card
            ));
        }

        self.prepared_cards = Some(cards);
        self.hand = mem::take(&mut self.hand)
            .into_iter()
            .zip(used_hand_indexes.into_iter())
            .filter_map(|(card, used)| (!used).then_some(card))
            .collect();

        Ok(())
    }
}
