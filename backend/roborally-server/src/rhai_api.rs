#[allow(clippy::wildcard_imports)]
use rhai::plugin::*;

#[export_module]
pub mod game_api {
    use rhai::EvalAltResult;
    use roborally_structs::position::Position as RoborallyPosition;
    use std::sync::{Arc, RwLock};

    use crate::game_state::GameState;

    pub type Game = Arc<RwLock<GameState>>;

    #[rhai_fn(pure, return_raw)]
    pub fn move_player_in_direction(
        game_lock: &mut Game,
        player_i: usize,
        direction: PlayerDirection,
    ) -> Result<MoveResult, Box<EvalAltResult>> {
        let mut game = game_lock.write().unwrap();
        let res = game
            .mov(player_i, direction)
            .map_err(Into::<Box<EvalAltResult>>::into)?;
        game.execute_reboots();
        Ok(res)
    }

    #[rhai_fn(pure, return_raw)]
    pub fn force_move_player_to(
        game_lock: &mut Game,
        player_i: usize,
        pos: RoborallyPosition,
        pushing_direction: PlayerDirection,
    ) -> Result<MoveResult, Box<EvalAltResult>> {
        let mut game = game_lock.write().unwrap();
        let res = game
            .force_move_to(player_i, pos, pushing_direction.into())
            .map_err(Into::<Box<EvalAltResult>>::into)?;
        game.execute_reboots();
        Ok(res)
    }

    #[rhai_fn(pure)]
    pub fn get_player_at_position(game: &mut Game, x: i64, y: i64) -> Option<usize> {
        game.read().unwrap().player_at_position(RoborallyPosition {
            x: x as i16,
            y: y as i16,
        })
    }

    #[rhai_fn(pure)]
    pub fn get_player_position(game: &mut Game, player_i: usize) -> Option<(i64, i64)> {
        game.read().unwrap().players.get(player_i).map(|p| {
            (
                i64::from(p.public_state.position.x),
                i64::from(p.public_state.position.y),
            )
        })
    }

    #[rhai_fn(pure, return_raw)]
    pub fn get_player_direction(
        game: &mut Game,
        player_i: usize,
    ) -> Result<PlayerDirection, Box<EvalAltResult>> {
        game.read()
            .unwrap()
            .players
            .get(player_i)
            .map(|p| p.public_state.direction)
            .ok_or_else(|| "There aren't that many players".into())
    }

    #[rhai_fn(pure, return_raw)]
    pub fn set_player_direction(
        game_lock: &mut Game,
        player_i: usize,
        direction: PlayerDirection,
    ) -> Result<(), Box<EvalAltResult>> {
        let mut game = game_lock.write().unwrap();
        let Some(p) = game.players.get_mut(player_i)
        else {
            return Err("There aren't that many players".into());
        };
        p.public_state.direction = direction;
        game.send_animation_item(&[], true);
        Ok(())
    }

    pub type MoveResult = crate::game_state::MoveResult;
    #[rhai_fn(get = "moved", pure)]
    pub fn move_result_get_moved(move_result: &mut MoveResult) -> bool {
        move_result.moved
    }

    #[rhai_fn(get = "reboot", pure)]
    pub fn move_result_get_reboot(move_result: &mut MoveResult) -> bool {
        move_result.reboot
    }

    pub type PlayerDirection = roborally_structs::position::ContinuousDirection;

    #[rhai_fn(name = "+", pure)]
    pub fn add_direction(direction: &mut PlayerDirection, increase: i64) -> PlayerDirection {
        *direction + increase
    }

    #[rhai_fn(name = "-", pure)]
    pub fn sub_direction(direction: &mut PlayerDirection, decrease: i64) -> PlayerDirection {
        *direction - decrease
    }

    #[rhai_fn(get = "direction", pure)]
    pub fn direction_get_direction(direction: &mut PlayerDirection) -> i64 {
        use roborally_structs::position::Direction::*;
        match (*direction).into() {
            Up => 0,
            Right => 1,
            Down => 2,
            Left => 3,
        }
    }
}
