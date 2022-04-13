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
#![warn(clippy::get_unwrap)]
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

use std::{
    collections::hash_map::{Entry, HashMap},
    str::FromStr,
    sync::Arc,
    time::Duration,
};

use futures::future::join_all;
use game::{Game, GamePhase};
use game_connection::PlayerConnection;
use http::StatusCode;
use roborally_structs::{game_map::GameMap, logging};
use serde::{Deserialize, Serialize};
use tokio::{sync::RwLock, time::Instant};
use warp::{reply::with_status, Filter, Reply};

#[derive(Deserialize)]
struct ConnectQuery {
    game_name: String,
    seat: usize,
    name: String,
}

async fn socket_connect_handler(
    query: ConnectQuery,
    ws: warp::ws::Ws,
    games_lock: Games,
) -> impl Reply {
    // It isn't possible to send an error response that can be reliably read in a browser during websocket handshake.
    // Therefore a connection is created even on invalid game_name, and the error is sent in Websocket close reason
    let game = games_lock
        .write()
        .await
        .get_mut(&query.game_name)
        .map(|entry| {
            entry.last_nobody_connected = None;
            Arc::clone(&entry.game)
        });
    ws.on_upgrade(move |socket| {
        PlayerConnection::create_and_start(game, socket, query.name, query.seat)
    })
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
    if name.len() > 50 {
        return with_status("Game name is too long".to_owned(), StatusCode::BAD_REQUEST);
    }
    let Some(map) = maps.get(&map_name)
    else {
        return with_status("Unknown map".to_owned(), StatusCode::BAD_REQUEST);
    };
    let game = match Game::new(map.clone(), players) {
        Ok(g) => g,
        Err(e) => return with_status(e, StatusCode::BAD_REQUEST),
    };
    let mut games = games_lock.write().await;
    match games.entry(name) {
        Entry::Occupied(_) => with_status(
            "Game with this name already exists".to_owned(),
            StatusCode::BAD_REQUEST,
        ),
        Entry::Vacant(vacant) => {
            vacant.insert(GameIndexEntry {
                map_name,
                last_nobody_connected: Some(Instant::now()),
                game: Arc::new(RwLock::new(game)),
            });
            with_status(String::new(), StatusCode::CREATED)
        }
    }
}

#[derive(Serialize)]
struct GameListItem {
    seats: Vec<Option<String>>,
    map_name: String,
    name: String,
}

enum GameListResult {
    ListItem(GameListItem),
    Cleanup(String),
}

async fn list_games_handler(games_lock: Games) -> impl Reply {
    let mut games = games_lock.write().await;
    let games_futures: Vec<_> = games
        .iter_mut()
        .map(async move |(game_name, index_entry)| {
            let game = index_entry.game.read().await;
            // while this cleans the game up from the list before all players leave,
            // that is not an issue - other players keep the game_arc reference as long
            // as they are connected, and no new connections are allowed anyways
            if let GamePhase::HasWinner(_) = game.phase {
                return GameListResult::Cleanup(game_name.clone());
            }
            let seats: Vec<Option<String>> = game
                .players
                .iter()
                .map(|player| {
                    player
                        .connected
                        .upgrade()
                        .map(|conn| conn.player_name.clone())
                })
                .collect();
            if seats.iter().all(Option::is_none) {
                if let Some(last_nobody_connected) = index_entry.last_nobody_connected {
                    if last_nobody_connected.elapsed() > Duration::from_secs(300) {
                        return GameListResult::Cleanup(game_name.clone());
                    }
                } else {
                    index_entry.last_nobody_connected = Some(Instant::now());
                }
            }
            GameListResult::ListItem(GameListItem {
                seats,
                map_name: index_entry.map_name.clone(),
                name: game_name.clone(),
            })
        })
        .collect();

    let mut games_list: Vec<_> = join_all(games_futures)
        .await
        .into_iter()
        .filter_map(|list_result| match list_result {
            GameListResult::ListItem(item) => Some(item),
            GameListResult::Cleanup(name) => {
                games.remove(&name);
                None
            }
        })
        .collect();
    games_list.sort_by(|a, b| a.name.cmp(&b.name));
    warp::reply::json(&games_list)
}

struct GameIndexEntry {
    map_name: String,
    last_nobody_connected: Option<Instant>,
    game: Arc<RwLock<Game>>,
}

type Games = Arc<RwLock<HashMap<String, GameIndexEntry>>>;
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

#[tokio::main]
async fn main() {
    logging::init();
    let games_lock: Games = Games::default();
    let maps: Maps = Arc::new(load_maps![
        "Test",
        "Dodge this",
        "Chop shop challenge",
        "Belt playground",
        "Burn run",
        "Sammy"
    ]);

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
        .map(|maps: Maps| {
            let mut maps_vec = maps.keys().collect::<Vec<_>>();
            maps_vec.sort();
            warp::reply::json(&maps_vec)
        });
    let get_map = api
        .and(warp::path("map").and(warp::path::end()))
        .and(warp::query::<GetMapQuery>())
        .and(warp::get())
        .and(create_maps_state())
        .map(|query: GetMapQuery, maps: Maps| {
            maps.get(&query.name).map_or_else::<Box<dyn Reply>, _, _>(
                || Box::new(with_status("Unknown map", StatusCode::NOT_FOUND)),
                |map| Box::new(rmp_serde::to_vec(map).unwrap()),
            )
        });
    let new_game = api
        .and(warp::path("new-game").and(warp::path::end()))
        .and(warp::post())
        .and(create_maps_state())
        .and(create_games_state())
        .and(warp::body::form::<NewGameData>())
        .then(new_game_handler);

    let socket = warp::path("websocket")
        .and(warp::path("game").and(warp::path::end()))
        .and(warp::query::<ConnectQuery>())
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
    eprintln!("Running at {:?}", ip_port);
    server.await;
}
