use std::{
    mem,
    sync::{Arc, Mutex, RwLock, Weak},
    time::Duration,
};

use rand::{prelude::SliceRandom, thread_rng};
use rhai::{exported_module, Engine, Scope, AST};
use roborally_structs::{
    card::Card,
    game_map::GameMap,
    game_state::{phase::RegisterMovePhase, GameStatusInfo},
};
use serde::Deserialize;
use tokio::time::Instant;

use crate::{game_state::GameState, player::Player, rhai_api::game_api};

#[derive(Deserialize)]
pub struct CardInitializationDefinition {
    pub asset: String,
    pub code: String,
    pub count: usize,
    pub name: String,
}

#[derive(Deserialize)]
pub struct NewGameData {
    pub map_name: String,
    pub name: String,
    player_count: usize,
    again_count: usize,
    card_definitions: Vec<CardInitializationDefinition>,
    round_registers: usize,
    draw_cards: usize,
}

pub struct Game {
    pub map: GameMap,
    /// Asset url, AST, scope
    pub cards: Vec<(String, Arc<AST>, Mutex<Scope<'static>>)>,
    pub last_nobody_connected: Mutex<Option<Instant>>,
    pub engine: Arc<Engine>,
    pub state: Arc<RwLock<GameState>>,
    /// Anything modifying the game state should lock this mutex before doing so.
    ///
    /// This is used alongside the state RwLock, because unfortunately .run() is unable to hold the RwLock the entire time
    running_guard: tokio::sync::Mutex<()>,
    pub log: Arc<Mutex<String>>,
    pub round_registers: usize,
    pub draw_cards: usize,
    pub player_count: usize,
    pub card_pack_size: usize,
}

impl Game {
    pub fn new(
        map: GameMap,
        NewGameData {
            map_name: _,
            name: _,
            player_count,
            again_count,
            card_definitions,
            round_registers,
            draw_cards,
        }: NewGameData,
    ) -> Result<Arc<Self>, String> {
        if map.spawn_points.len() < player_count {
            return Err("Not enough spawn points on map".to_owned());
        }

        if round_registers > draw_cards {
            return Err("Too few cards to draw".to_owned());
        }

        if round_registers < 1 {
            return Err("Too few registers per round".to_owned());
        }

        if again_count + card_definitions.iter().map(|c| c.count).sum::<usize>() <= draw_cards + 1 {
            return Err("Too many cards to draw".to_owned());
        }

        let mut spawn_points = map.spawn_points.clone();
        let (shuffled_spawn_points, _) =
            spawn_points.partial_shuffle(&mut thread_rng(), player_count);

        let players: Vec<Player> = shuffled_spawn_points
            .iter()
            .map(|sp| Player::new(*sp, again_count, &card_definitions, draw_cards))
            .collect();

        let state = Arc::new(RwLock::new(GameState {
            status: GameStatusInfo::Programming,
            players,
            game: Weak::new(),
            winner: None,
            reboot_queue: Vec::new(),
            running_state: (0, RegisterMovePhase::Checkpoints),
        }));

        let mut engine = Engine::new();
        engine.set_max_operations(20000);
        engine.register_global_module(exported_module!(game_api).into());
        let log = Arc::new(Mutex::new(String::new()));
        {
            let log = Arc::clone(&log);
            engine.on_print(move |msg| log.lock().unwrap().push_str(msg));
        }
        {
            let log = Arc::clone(&log);
            engine.on_debug(move |msg, src, pos| {
                log.lock()
                    .unwrap()
                    .push_str(&format!("{} @ {pos:?} > {msg}", src.unwrap()));
            });
        }

        let mut game = Game {
            map,
            cards: Vec::with_capacity(card_definitions.len()),
            last_nobody_connected: Mutex::new(Some(Instant::now() + Duration::from_secs(60))),
            engine: Arc::new(engine),
            state,
            running_guard: tokio::sync::Mutex::new(()),
            log,
            round_registers,
            draw_cards,
            player_count,
            card_pack_size: again_count + card_definitions.iter().map(|c| c.count).sum::<usize>(),
        };

        for CardInitializationDefinition {
            asset,
            code,
            count: _,
            name: card_name,
        } in card_definitions
        {
            let scope = game.create_scope();
            let mut ast = game
                .engine
                .compile_with_scope(&scope, code)
                .map_err(|e| format!("Error compiling script for card {card_name}: {e}"))?;
            ast.set_source(card_name);
            game.cards.push((asset, Arc::new(ast), Mutex::new(scope)));
        }

        let game = Arc::new(game);
        game.state.try_write().unwrap().game = Arc::downgrade(&game);
        Ok(game)
    }

    fn create_scope(&self) -> Scope<'static> {
        let mut scope = Scope::new();
        scope.push_constant("PLAYER_COUNT", self.player_count as i64);
        scope.push_constant("ROUND_REGISTERS", self.round_registers as i64);
        scope.push_constant("MAP_WIDTH", self.map.tiles.size().x as i64);
        scope.push_constant("MAP_HEIGHT", self.map.tiles.size().y as i64);
        scope.push_constant("GAME", Arc::clone(&self.state));
        scope
    }

    /// Handle when a player submits their programmed registers for given round
    pub async fn program(&self, seat: usize, cards: Vec<Card>) -> Result<(), String> {
        if cards.len() != self.round_registers {
            return Err("Wrong number of cards".to_owned());
        }

        let _guard = self.running_guard.lock().await;

        let mut state = self.state.write().unwrap();
        state.players[seat].program(cards)?;
        state.send_programming_state_to_all();

        let should_run = state.players.iter().all(|p| p.prepared_cards.is_some());
        drop(state);
        if should_run {
            self.run();
        }
        Ok(())
    }

    /// Execute player's card in given register
    ///
    /// If it's a SPAM/Again, this recurses appropriately
    ///
    /// All reboots are executed and state updates sent
    fn execute_card(&self, player_i: usize, register_i: usize) {
        use Card::*;
        if self.state.read().unwrap().players[player_i]
            .public_state
            .is_rebooting
        {
            return;
        }

        let mut execute_register_i = register_i;
        let mut state = self.state.write().unwrap();
        loop {
            let player = &mut state.players[player_i];
            let card = player.prepared_cards.as_ref().unwrap()[execute_register_i];
            match card {
                Again => {
                    if execute_register_i == 0 {
                        // Push the again to discard pile, replace with SPAM to reuse logic for drawing a replacement card
                        player.discard_pile.push(mem::replace(
                            &mut player.prepared_cards.as_mut().unwrap()[execute_register_i],
                            SPAM,
                        ));
                        continue;
                    }

                    execute_register_i -= 1;
                }
                SPAM => {
                    player.prepared_cards.as_mut().unwrap()[execute_register_i] =
                        player.draw_one_card();
                    // show the replaced card
                    state.send_animation_item(&[], true);
                    continue;
                }
                Custom(card_i) => {
                    let ast = Arc::clone(&self.cards[card_i].1);
                    let engine = Arc::clone(&self.engine);
                    drop(state);
                    let res = engine.call_fn::<()>(
                        &mut self.cards[card_i].2.lock().unwrap(),
                        &ast,
                        "execute",
                        (player_i as i64, register_i as i64),
                    );
                    if let Err(e) = res {
                        self.log.lock().unwrap().push_str(&format!(
                            "Error running card {} on register {} for player {}: {}\n",
                            ast.source().unwrap(),
                            register_i + 1,
                            player_i,
                            e
                        ));
                    }
                    break;
                }
            }
        }
    }

    fn run(&self) {
        use RegisterMovePhase::*;
        for register_i in 0..self.round_registers {
            for register_phase in RegisterMovePhase::ORDER {
                let mut state = self.state.write().unwrap();
                state.running_state = (register_i, register_phase);
                state.send_animation_item(&[], true);
                match register_phase {
                    PlayerCards => {
                        let indices = state.player_indices_by_priority();
                        drop(state);
                        for player_i in indices {
                            self.execute_card(player_i, register_i);
                        }
                        state = self.state.write().unwrap();
                    }
                    FastBelts => {
                        state.execute_belts(true);
                        state.execute_belts(true);
                    }
                    SlowBelts => state.execute_belts(false),
                    PushPanels => state.execute_push_panels(register_i),
                    Rotations => state.execute_rotators(),
                    Lasers => state.execute_lasers(),
                    Checkpoints => state.execute_checkpoints(),
                }
                let log = mem::take(&mut *self.log.lock().unwrap());
                if !log.is_empty() {
                    state.send_log(&log);
                }
            }
        }

        let mut state = self.state.write().unwrap();
        for player in &mut state.players {
            player
                .discard_pile
                .append(&mut player.prepared_cards.take().unwrap());
            player.discard_pile.append(&mut player.hand);
            player.hand = player.draw_n_cards(self.draw_cards);
            player.public_state.is_rebooting = false;
        }
        state.status = GameStatusInfo::Programming;
        state.send_programming_state_to_all();
        state.send_general_state();
    }
}
