#![warn(clippy::nursery)]
#![allow(clippy::use_self)]
#![warn(clippy::pedantic)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_possible_wrap)]
#![allow(clippy::cast_possible_truncation)]
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
#![warn(clippy::shadow_same)]
#![warn(clippy::shadow_unrelated)]
#![warn(clippy::str_to_string)]
#![warn(clippy::string_add)]
#![warn(clippy::string_to_string)]
#![warn(clippy::try_err)]
// features
#![feature(pattern)]
#![feature(const_precise_live_drops)]
#![feature(async_closure)]
#![feature(array_zip)]
#![feature(never_type)]
#![feature(let_chains)]
#![feature(is_some_and)]
#![feature(iter_intersperse)]

mod game;
mod game_connection;
mod game_state;
mod parser;
mod player;
mod rhai_api;

use std::{
    collections::hash_map::{Entry, HashMap},
    fs,
    io::Read,
    mem,
    str::FromStr,
    sync::Arc,
    time::Duration,
};

use game::{Game, NewGameData};
use game_connection::PlayerConnection;
use http::StatusCode;
use roborally_structs::{
    game_map::GameMap,
    logging::{self, info},
};
use serde::{Deserialize, Serialize};
use tokio::{select, sync::RwLock, time::Instant};
use warp::{reply::with_status, Filter, Reply};

use crate::{game_connection::SocketMessage, parser::Parse};

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
    let game = games_lock.read().await.get(&query.game_name).map(|g| {
        *g.last_nobody_connected.lock().unwrap() = None;
        Arc::clone(g)
    });
    ws.on_upgrade(move |socket| {
        PlayerConnection::create_and_start(game, socket, query.name, query.seat)
    })
}

async fn new_game_handler(maps: Maps, games_lock: Games, mut data: NewGameData) -> impl Reply {
    let game_name = mem::take(&mut data.name);
    if game_name.len() > 50 {
        return with_status("Game name is too long".to_owned(), StatusCode::BAD_REQUEST);
    }
    let Some(map) = maps.get(&data.map_name)
    else {
        return with_status("Unknown map".to_owned(), StatusCode::BAD_REQUEST);
    };
    let game = match Game::new(map.clone(), data) {
        Ok(g) => g,
        Err(e) => return with_status(e, StatusCode::BAD_REQUEST),
    };
    let mut games = games_lock.write().await;
    match games.entry(game_name) {
        Entry::Occupied(_) => with_status(
            "Game with this name already exists".to_owned(),
            StatusCode::BAD_REQUEST,
        ),
        Entry::Vacant(vacant) => {
            vacant.insert(game);
            with_status(String::new(), StatusCode::CREATED)
        }
    }
}

#[derive(Serialize)]
struct GameListItem {
    seats: Vec<Option<String>>,
    name: String,
    map_name: String,
    cards_assets_names: Vec<(String, String)>,
    card_pack_size: usize,
    round_registers: usize,
    draw_cards: usize,
}

async fn list_games_handler(games_lock: Games) -> impl Reply {
    let mut games = games_lock.write().await;
    let mut games_list = Vec::new();
    games.retain(|name, game| {
        if game
            .last_nobody_connected
            .lock()
            .unwrap()
            .is_some_and(|t| t.elapsed() > Duration::from_secs(300))
        {
            return false;
        }
        let seats: Vec<Option<String>> = game
            .player_connections
            .iter()
            .map(|player| {
                player
                    .read()
                    .unwrap()
                    .upgrade()
                    .map(|conn| conn.player_name.clone())
            })
            .collect();
        if seats.iter().all(Option::is_none) {
            let mut last_nobody_connected_guard = game.last_nobody_connected.lock().unwrap();
            if let Some(last_nobody_connected) = *last_nobody_connected_guard {
                if last_nobody_connected.elapsed() > Duration::from_secs(300) {
                    return false;
                }
            } else {
                *last_nobody_connected_guard = Some(Instant::now());
            }
        }
        games_list.push(GameListItem {
            seats,
            map_name: game.map.name.clone(),
            name: name.clone(),
            cards_assets_names: [
                ("/assets/again.png".to_owned(), "Again".to_owned()),
                ("/assets/spam.png".to_owned(), "SPAM".to_owned()),
            ]
            .into_iter()
            .chain(
                game.cards
                    .iter()
                    .map(|c| (c.0.clone(), c.1.source().unwrap().to_owned())),
            )
            .collect(),
            card_pack_size: game.card_pack_size,
            round_registers: game.round_registers,
            draw_cards: game.draw_cards,
        });
        true
    });

    games_list.sort_by(|a, b| a.name.cmp(&b.name));
    warp::reply::json(&games_list)
}

type Games = Arc<RwLock<HashMap<String, Arc<Game>>>>;
type Maps = Arc<HashMap<String, GameMap>>;

#[derive(Deserialize)]
struct GetMapQuery {
    name: String,
}

#[tokio::main]
#[allow(clippy::too_many_lines)]
async fn main() {
    logging::init();
    let games_lock: Games = Games::default();
    let maps: Maps = Arc::new(
        fs::read_dir("maps")
            .unwrap()
            .map(|entry| {
                let mut buffer = String::new();
                fs::File::open(entry.unwrap().path())
                    .unwrap()
                    .read_to_string(&mut buffer)
                    .unwrap();
                let map = GameMap::parse(&buffer, "").unwrap();
                (map.name.clone(), map)
            })
            .collect(),
    );

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
    #[allow(clippy::shadow_unrelated)]
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
        .and(warp::body::json::<NewGameData>())
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
    let ip_port = std::env::var("PORT")
        .ok()
        .and_then(|p| u16::from_str(&p).ok())
        .map_or(([127, 0, 0, 1], 8080), |p| ([0, 0, 0, 0], p));
    let server = warp::serve(routes);
    #[cfg(unix)]
    let mut term =
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()).unwrap();
    #[cfg(windows)]
    let mut term = FakeTerm;

    info!("Running at {ip_port:?}");
    server
        .bind_with_graceful_shutdown(ip_port, async move {
            select! {
                _ = tokio::signal::ctrl_c() => (),
                _ = term.recv() => (),
            }
            for game in games_lock.read().await.values() {
                for player in &game.player_connections {
                    if let Some(conn) = player.read().unwrap().upgrade() {
                        conn.sender
                            .send(SocketMessage::CloseWithNotice(
                                "Server is shutting down. Sorry :(".to_owned(),
                            ))
                            .unwrap();
                    }
                }
            }
        })
        .1
        .await;
}

#[cfg(windows)]
struct FakeTerm;

#[cfg(windows)]
impl FakeTerm {
    async fn recv(&mut self) -> ! {
        futures::future::pending().await
    }
}
