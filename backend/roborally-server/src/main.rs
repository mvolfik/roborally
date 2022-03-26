#![warn(clippy::nursery)]
#![allow(clippy::use_self)]
#![warn(clippy::pedantic)]
#![allow(clippy::unused_unit)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::enum_glob_use)]
#![allow(clippy::many_single_char_names)]
#![warn(clippy::shadow_reuse)]
#![warn(clippy::shadow_same)]
#![warn(clippy::shadow_unrelated)]
#![feature(pattern)]
#![feature(const_precise_live_drops)]
#![feature(let_else)]

mod game;
mod game_connection;
mod parser;

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use crate::{game::Game, game_connection::GameConnection};

use actix_files::Files;
use actix_web::{
    error::ErrorBadRequest, http::header::ContentType, web, App, Error, HttpRequest, HttpResponse,
    HttpServer,
};
use actix_web_actors::ws;
use rand::random;
use roborally_structs::{game_map::GameMap, transport::ConnectInfo};
use serde::{Deserialize, Serialize};

async fn game_handler(
    state: web::Data<AppState>,
    req: HttpRequest,
    info: web::Query<ConnectInfo>,
    stream: web::Payload,
) -> Result<HttpResponse, Error> {
    let actor = { GameConnection::new(&mut state.games.lock().unwrap(), info.0)? };
    ws::start(actor, &req, stream)
}

#[derive(Deserialize)]
struct NewGameData {
    players: usize,
    map_name: String,
    name: String,
}

async fn new_game_handler(
    state: web::Data<AppState>,
    info: web::Form<NewGameData>,
) -> Result<HttpResponse, Error> {
    let NewGameData {
        players,
        map_name,
        name,
    } = info.0;
    let map = state
        .maps
        .get(&map_name)
        .ok_or_else(|| ErrorBadRequest("unknown map"))?
        .clone();
    let game = Arc::new(Mutex::new(
        Game::new(map, players, name).map_err(|e| ErrorBadRequest(e))?,
    ));

    let mut id = random();
    {
        let mut games = state.games.lock().unwrap();
        {
            while games.contains_key(&id) {
                id = random();
            }
        }
        games.insert(id, game);
    }
    Ok(HttpResponse::Ok()
        .content_type(ContentType::plaintext())
        .body(id.to_string()))
}

#[derive(Serialize)]
struct GameListItem {
    id: String,
    players_connected: u8,
    players_n: u8,
    name: String,
}

async fn list_games_handler(state: web::Data<AppState>) -> web::Json<Vec<GameListItem>> {
    let games = state.games.lock().unwrap();
    let games_list: Vec<_> = games
        .iter()
        .map(|(id, game_mutex)| {
            let game = game_mutex.lock().unwrap();
            GameListItem {
                id: id.to_string(),
                players_connected: 0,
                players_n: game.players.len() as u8,
                name: game.name.clone(),
            }
        })
        .collect();
    web::Json(games_list)
}

#[derive(Default)]
struct AppState {
    games: Mutex<HashMap<u64, Arc<Mutex<Game>>>>,
    maps: HashMap<String, GameMap>,
}

macro_rules! load_maps {
    [ $( $name: literal ),* ] => {
        {
            use crate::parser::{Parse, ParseError};
            HashMap::from([
                $(
                    (
                        $name.to_string(),
                        GameMap::parse(include_str!(concat!("../../../maps/", $name)), concat!("Map(", $name, ")"))
                            .map_err(ParseError::get)
                            .unwrap(),
                    ),
                )*
            ])
        }
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let maps = load_maps!["test.csv"];
    // Shared game state. web::Data uses Arc internally, so we create state outside the server factory, and the factory clones the Arc for each thread
    let state = web::Data::new(AppState {
        games: Default::default(),
        maps,
    });

    let server = HttpServer::new(move || {
        App::new()
            .app_data(state.clone())
            .route("/websocket/game", web::get().to(game_handler))
            .route("/api/list-games", web::get().to(list_games_handler))
            .route("/api/new-game", web::post().to(new_game_handler))
            .service(Files::new("/", "../roborally-frontend/dist").index_file("index.html"))
    })
    .bind(("127.0.0.1", 8080))?;
    println!("Running");
    server.run().await
}
