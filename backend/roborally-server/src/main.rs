#![warn(clippy::nursery)]
#![allow(clippy::use_self)]
#![warn(clippy::pedantic)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::unused_unit)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::enum_glob_use)]
#![allow(clippy::many_single_char_names)]
// restrictions
#![warn(clippy::allow_attributes_without_reason)]
#![warn(clippy::clone_on_ref_ptr)]
#![warn(clippy::else_if_without_else)]
// enable later: #![warn(clippy::get_unwrap)]
#![warn(clippy::if_then_some_else_none)]
#![warn(clippy::let_underscore_must_use)]
#![warn(clippy::shadow_reuse)]
#![warn(clippy::shadow_same)]
#![warn(clippy::shadow_unrelated)]
#![warn(clippy::str_to_string)]
#![warn(clippy::string_add)]
#![warn(clippy::string_to_string)]
#![warn(clippy::try_err)]
// features
#![feature(pattern)]
#![feature(const_precise_live_drops)]
#![feature(let_else)]
#![feature(async_closure)]
#![feature(array_zip)]
#![feature(never_type)]
#![feature(label_break_value)]
#![feature(let_chains)]
#![feature(is_some_with)]
#![feature(iter_intersperse)]

mod game;
mod game_connection;
mod parser;

use std::{collections::HashMap, str::FromStr, sync::Arc, time::Duration};

use futures::future::join_all;
use game::Game;
use game_connection::PlayerConnection;
use http::StatusCode;
use rand::random;
use roborally_structs::{game_map::GameMap, logging};
use serde::{Deserialize, Serialize};
use tokio::{sync::RwLock, time::Instant};
use warp::{reply::with_status, Filter, Reply};

async fn socket_connect_handler(game_id: u64, ws: warp::ws::Ws, games_lock: Games) -> impl Reply {
    let game = games_lock.write().await.get_mut(&game_id).map(|entry| {
        entry.last_nobody_connected = None;
        Arc::clone(&entry.game)
    });
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
    games_lock: Games,
    NewGameData {
        players,
        map_name,
        name,
    }: NewGameData,
) -> impl Reply {
    let Some(map) = maps.get(&map_name)
    else {
        return with_status("Unknown map".to_owned(), StatusCode::BAD_REQUEST)
    };
    let game = match Game::new(map.clone(), players) {
        Ok(g) => g,
        Err(e) => return with_status(e, StatusCode::BAD_REQUEST),
    };

    let mut id = random();
    {
        let mut games = games_lock.write().await;
        {
            while games.contains_key(&id) {
                id = random();
            }
        }
        games.insert(
            id,
            GameIndexEntry {
                name,
                map_name,
                last_nobody_connected: Some(Instant::now()),
                game: Arc::new(RwLock::new(game)),
            },
        );
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
    let futures = games
        .iter()
        .map(async move |(id, index_entry)| GameListItem {
            id: id.to_string(),
            seats: index_entry
                .game
                .read()
                .await
                .players
                .iter()
                .map(|p| p.connected.upgrade().map(|c| c.name.clone()))
                .collect(),
            map_name: index_entry.map_name.clone(),
            name: index_entry.name.clone(),
        });
    warp::reply::json(&join_all(futures).await)
}

struct GameIndexEntry {
    name: String,
    map_name: String,
    last_nobody_connected: Option<Instant>,
    game: Arc<RwLock<Game>>,
}

type Games = Arc<RwLock<HashMap<u64, GameIndexEntry>>>;
type Maps = Arc<HashMap<String, GameMap>>;

macro_rules! load_maps {
    [ $( $name: literal ),* ] => {
        {
            use crate::parser::Parse;
            HashMap::from([
                $(
                    (
                        $name.to_owned(),
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

#[allow(clippy::needless_pass_by_value)]
fn handle_get_map(query: GetMapQuery, maps: Maps) -> Box<dyn Reply> {
    maps.get(&query.name).map_or_else::<Box<dyn Reply>, _, _>(
        || Box::new(with_status("Unknown map", StatusCode::NOT_FOUND)),
        |map| Box::new(rmp_serde::to_vec(map).unwrap()),
    )
}

#[tokio::main]
async fn main() {
    logging::init();
    let games_lock: Games = Games::default();
    let maps: Maps = Arc::new(load_maps!["Test", "Dodge this", "Chop shop challenge"]);

    // state is a allow-anything "filter" which clones the games Arc and passes it as a context
    let create_games_state = || {
        let arc = Arc::clone(&games_lock);
        warp::any().map(move || Arc::clone(&arc))
    };

    let create_maps_state = || {
        let arc = Arc::clone(&maps);

        warp::any().map(move || Arc::clone(&arc))
    };

    let api = warp::path("api");
    let list_games = api
        .and(warp::path("list-games").and(warp::path::end()))
        .and(warp::get())
        .and(create_games_state())
        .then(list_games_handler);
    #[allow(clippy::shadow_unrelated)]
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

    let static_files = warp::fs::dir("www");

    let routes = list_games
        .or(list_maps)
        .or(get_map)
        .or(new_game)
        .or(socket)
        .or(static_files);
    let ip_port = match std::env::var("PORT")
        .ok()
        .and_then(|p| u16::from_str(&p).ok())
    {
        Some(p) => ([0, 0, 0, 0], p),
        None => ([127, 0, 0, 1], 8080),
    };
    let server = warp::serve(routes).run(ip_port);
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(5)).await;
            let mut games = games_lock.write().await;
            let to_remove = games.iter_mut().map(async move |(id, index_entry)| {
                if index_entry
                    .game
                    .read()
                    .await
                    .players
                    .iter()
                    .any(|p| p.connected.strong_count() > 0)
                {
                    None
                } else if let Some(last_nobody_connected) = index_entry.last_nobody_connected {
                    #[allow(clippy::if_then_some_else_none)] // that's unreadable
                    if Instant::now().duration_since(last_nobody_connected)
                        > Duration::from_secs(60)
                    {
                        Some(*id)
                    } else {
                        None
                    }
                } else {
                    index_entry.last_nobody_connected = Some(Instant::now());
                    None
                }
            });
            for id in join_all(to_remove).await.into_iter().flatten() {
                games.remove(&id);
            }
        }
    });
    eprintln!("Running at {:?}", ip_port);
    server.await;
}
