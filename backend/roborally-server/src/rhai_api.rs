#[allow(clippy::wildcard_imports)]
use rhai::plugin::*;

#[export_module]
pub mod game_api {
    use rhai::EvalAltResult;
    pub type Game = std::sync::Arc<std::sync::RwLock<crate::game_state::GameState>>;

    #[rhai_fn(pure, return_raw)]
    pub fn move_player_in_direction(
        game_lock: &mut Game,
        player_i: i64,
        direction: PlayerDirection,
    ) -> Result<MoveResult, Box<EvalAltResult>> {
        let mut game = game_lock.write().unwrap();
        if player_i as usize >= game.players.len() {
            return Err("There aren't that many players".into());
        }
        let (res, animations) = game.mov(player_i as usize, direction);
        game.execute_reboots(&animations);
        Ok(res)
    }

    #[rhai_fn(pure, return_raw)]
    pub fn force_move_player_to(
        game_lock: &mut Game,
        player_i: i64,
        pos: MapPosition,
        pushing_direction: PlayerDirection,
    ) -> Result<MoveResult, Box<EvalAltResult>> {
        let mut game = game_lock.write().unwrap();
        if player_i as usize >= game.players.len() {
            return Err("There aren't that many players".into());
        }
        let res = game.force_move_to(player_i as usize, pos, pushing_direction.into());
        game.execute_reboots(&[]);
        Ok(res)
    }

    #[rhai_fn(pure)]
    pub fn get_player_at_position(game: &mut Game, pos: MapPosition) -> Dynamic {
        game.read()
            .unwrap()
            .player_at_position(pos)
            .map(|p| Dynamic::from_int(p as i64))
            .unwrap_or_default()
    }

    #[rhai_fn(pure, return_raw)]
    pub fn get_player_position(
        game: &mut Game,
        player_i: i64,
    ) -> Result<MapPosition, Box<EvalAltResult>> {
        game.read()
            .unwrap()
            .players
            .get(player_i as usize)
            .map(|p| p.public_state.position)
            .ok_or_else(|| "There aren't that many players".into())
    }

    #[rhai_fn(pure, return_raw)]
    pub fn get_player_direction(
        game: &mut Game,
        player_i: i64,
    ) -> Result<PlayerDirection, Box<EvalAltResult>> {
        game.read()
            .unwrap()
            .players
            .get(player_i as usize)
            .map(|p| p.public_state.direction)
            .ok_or_else(|| "There aren't that many players".into())
    }

    #[rhai_fn(pure, return_raw)]
    pub fn set_player_direction(
        game_lock: &mut Game,
        player_i: i64,
        direction: PlayerDirection,
    ) -> Result<(), Box<EvalAltResult>> {
        let mut game = game_lock.write().unwrap();
        let Some(p) = game.players.get_mut(player_i as usize)
        else {
            return Err("There aren't that many players".into());
        };
        p.public_state.direction = direction;
        game.send_animation_item(&[], true);
        Ok(())
    }

    #[rhai_fn(pure)]
    pub fn is_void_at(game: &mut Game, pos: MapPosition) -> bool {
        !game
            .read()
            .unwrap()
            .game
            .upgrade()
            .unwrap()
            .map
            .tiles
            .get(pos)
            .is_some_and(|t| t.typ != roborally_structs::tile_type::TileType::Void)
    }

    pub type MoveResult = crate::game_state::MoveResult;
    #[rhai_fn(get = "moved", pure)]
    pub fn move_result_get_moved(move_result: &mut MoveResult) -> bool {
        move_result.moved
    }

    #[rhai_fn(get = "rebooted", pure)]
    pub fn move_result_get_rebooted(move_result: &mut MoveResult) -> bool {
        move_result.rebooted
    }

    pub type PlayerDirection = roborally_structs::position::ContinuousDirection;

    pub fn direction_up() -> PlayerDirection {
        roborally_structs::position::Direction::Up.to_continuous()
    }

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

    pub type MapPosition = roborally_structs::position::Position;

    pub fn position_from_xy(x: i64, y: i64) -> MapPosition {
        MapPosition {
            x: x as i16,
            y: y as i16,
        }
    }

    #[rhai_fn(get = "x", pure)]
    pub fn position_get_x(pos: &mut MapPosition) -> i64 {
        pos.x as i64
    }

    #[rhai_fn(get = "y", pure)]
    pub fn position_get_y(pos: &mut MapPosition) -> i64 {
        pos.y as i64
    }

    #[rhai_fn(name = "+", pure)]
    pub fn add_position_direction(position: &mut MapPosition, dir: PlayerDirection) -> MapPosition {
        position.moved_in_direction(dir.into())
    }
}
