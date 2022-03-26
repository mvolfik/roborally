use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use actix::{Actor, ActorContext, StreamHandler};
use actix_web::{error::ErrorBadRequest, Error};
use actix_web_actors::ws;
use roborally_structs::transport::{ConnectInfo, ServerMessage};

use crate::game::Game;

#[derive(Debug)]
pub struct GameConnection {
    pub game: Arc<Mutex<Game>>,
    pub player_i: usize,
    _prevent_construct: (),
}

impl GameConnection {
    #[must_use]
    pub fn new(games: &HashMap<u64, Arc<Mutex<Game>>>, info: ConnectInfo) -> Result<Self, Error> {
        let game_arc = games
            .get(&info.game_id)
            .ok_or_else(|| ErrorBadRequest("invalid game id"))?;
        let conn = Self {
            game: game_arc.clone(),
            player_i: info.player_i,
            _prevent_construct: (),
        };

        let mut game = game_arc.lock().unwrap();
        game.players
            .get_mut(info.player_i)
            .ok_or_else(|| ErrorBadRequest("player_i out of range"))?
            .connected = Some((info.name().to_string(), ()));

        // todo : share connection somehow
        Ok(conn)
    }
}

impl Actor for GameConnection {
    type Context = ws::WebsocketContext<Self>;
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for GameConnection {
    fn handle(&mut self, _msg: Result<ws::Message, ws::ProtocolError>, _ctx: &mut Self::Context) {
        // match msg {
        //     Ok(ws::Message::Ping(msg)) => ctx.pong(&msg),
        //     Ok(ws::Message::Text(text)) => ctx.text(text),
        //     Ok(ws::Message::Binary(bin)) => ctx.binary(bin),
        //     _ => (),
        // }
        todo!()
    }

    fn started(&mut self, ctx: &mut Self::Context) {
        let game = self.game.lock().unwrap();
        let Some(state) = game
        .get_state_for_player(self.player_i) else {
            ctx.close(None);
            return;
        };

        let Ok(msg) = rmp_serde::to_vec(&ServerMessage::InitInfo {
            map: game.map.clone(),
            state,
        }) else {
            ctx.close(None);
            return;
        };

        ctx.binary(msg);
    }

    fn finished(&mut self, ctx: &mut Self::Context) {
        ctx.stop();
    }
}
