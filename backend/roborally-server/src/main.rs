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
#![feature(async_closure)]
#![feature(array_zip)]
#![feature(never_type)]
#![feature(label_break_value)]
#![feature(let_chains)]

mod game;
mod game_connection;
mod parser;

use std::{collections::HashMap, str::FromStr, sync::Arc};

use futures::future::join_all;
use game::Game;
use game_connection::PlayerConnection;
use http::StatusCode;
use rand::random;
use roborally_structs::{game_map::GameMap, logging};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use warp::{reply::with_status, Filter, Reply};

async fn socket_connect_handler(game_id: u64, ws: warp::ws::Ws, games_lock: Games) -> impl Reply {
    let game = games_lock.read().await.get(&game_id).map(|x| x.1.clone());
    ws.on_upgrade(move |socket| PlayerConnection::create_and_start(game, socket))
}

#[derive(Deserialize)]
struct NewGameData {
    players: usize,
    map_name: String,
    name: String,
}

async fn new_game_handler(
    maps: Maps,
    games: Games,
    NewGameData {
        players,
        map_name,
        name,
    }: NewGameData,
) -> impl Reply {
    let Some(map) = maps.get(&map_name) else {
        return with_status("Unknown map".to_string(), StatusCode::BAD_REQUEST)
    };
    let game = match Game::new(map.clone(), players, name) {
        Ok(g) => g,
        Err(e) => return with_status(e, StatusCode::BAD_REQUEST),
    };

    let mut id = random();
    {
        let mut games = games.write().await;
        {
            while games.contains_key(&id) {
                id = random();
            }
        }
        games.insert(id, (map_name, Arc::new(RwLock::new(game))));
    }
    with_status(id.to_string(), StatusCode::CREATED)
}

#[derive(Serialize)]
struct GameListItem {
    id: String,
    seats: Vec<Option<String>>,
    map_name: String,
    name: String,
}

async fn list_games_handler(games_lock: Games) -> impl Reply {
    let games = games_lock.read().await;
    let futures: Vec<_> = games
        .iter()
        .map(async move |(id, (map_name, game_lock))| {
            let game = game_lock.read().await;
            GameListItem {
                id: id.to_string(),
                seats: game
                    .players
                    .iter()
                    .map(|p| p.connected.upgrade().map(|c| c.name.clone()))
                    .collect(),
                map_name: map_name.clone(),
                name: game.name.clone(),
            }
        })
        .collect();
    warp::reply::json(&join_all(futures).await)
}

type Games = Arc<RwLock<HashMap<u64, (String, Arc<RwLock<Game>>)>>>;
type Maps = Arc<HashMap<String, GameMap>>;

macro_rules! load_maps {
    [ $( $name: literal ),* ] => {
        {
            use crate::parser::Parse;
            HashMap::from([
                $(
                    (
                        $name.to_string(),
                        GameMap::parse(include_str!(concat!("../../../maps/", $name)), concat!("Map(", $name, ")"))
                            .unwrap(),
                    ),
                )*
            ])
        }
    }
}

#[derive(Deserialize)]
struct GetMapQuery {
    name: String,
}

fn handle_get_map(query: GetMapQuery, maps: Maps) -> Box<dyn Reply> {
    if let Some(map) = maps.get(&query.name) {
        Box::new(rmp_serde::to_vec(map).unwrap())
    } else {
        Box::new(with_status("Unknown map", StatusCode::NOT_FOUND))
    }
}

#[tokio::main]
async fn main() {
    logging::init();
    let maps: Maps = Arc::new(load_maps!["test.csv"]);
    // Shared game state. web::Data uses Arc internally, so we create state outside the server factory, and the factory clones the Arc for each thread

    let games: Games = Games::default();
    // state is a allow-anything "filter" which clones the games Arc and passes it as a context
    let create_games_state = move || {
        let arc = games.clone();
        warp::any().map(move || arc.clone())
    };
    let create_maps_state = move || {
        let arc = maps.clone();
        warp::any().map(move || arc.clone())
    };
    let api = warp::path("api");
    let list_games = api
        .and(warp::path("list-games").and(warp::path::end()))
        .and(warp::get())
        .and(create_games_state())
        .then(list_games_handler);
    let list_maps = api
        .and(warp::path("list-maps").and(warp::path::end()))
        .and(warp::get())
        .and(create_maps_state())
        .map(|maps: Maps| warp::reply::json(&maps.keys().collect::<Vec<_>>()));
    let get_map = api
        .and(warp::path("map").and(warp::path::end()))
        .and(warp::get())
        .and(warp::query::<GetMapQuery>())
        .and(create_maps_state())
        .map(handle_get_map);
    let new_game = api
        .and(warp::path("new-game").and(warp::path::end()))
        .and(warp::post())
        .and(create_maps_state())
        .and(create_games_state())
        .and(warp::body::form::<NewGameData>())
        .then(new_game_handler);

    let socket = warp::path!("websocket" / "game" / u64)
        .and(warp::ws())
        .and(create_games_state())
        .then(socket_connect_handler);

    let static_files = warp::fs::dir("../roborally-frontend/dist");

    let routes = list_games
        .or(list_maps)
        .or(get_map)
        .or(new_game)
        .or(socket)
        .or(static_files);
    let ip_port = match std::env::var("PORT")
        .ok()
        .map(|p| u16::from_str(&p).ok())
        .flatten()
    {
        Some(p) => ([0, 0, 0, 0], p),
        None => ([127, 0, 0, 1], 8080),
    };
    let server = warp::serve(routes).run(ip_port);
    eprintln!("Running at {:?}", ip_port);
    server.await
}
