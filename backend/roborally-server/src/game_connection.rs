use std::{sync::Arc, time::Duration};

use futures::{stream::SplitSink, SinkExt, Stream, StreamExt};
use roborally_structs::{
    logging::{debug, error, info, warn},
    transport::{ClientMessage, ServerMessage},
};
use tokio::{sync::RwLock, time::timeout};
use warp::ws::{Message, WebSocket};

use crate::game::{run_moving_phase, Game, GamePhase};

#[derive(Debug)]
pub struct SocketWriter(SplitSink<WebSocket, Message>);

impl SocketWriter {
    pub async fn close_with_notice(&mut self, msg: String) {
        info!("Closing connection with message: {}", &msg);
        self.send_message(ServerMessage::Notice(msg)).await;
        if let Err(e) = self.0.send(Message::close()).await {
            warn!("Error when closing connection: {}", e);
        }
    }

    pub async fn send_message(&mut self, msg: ServerMessage) {
        if let Ok(vec) = rmp_serde::to_vec(&msg) {
            if let Err(e) = self.0.send(Message::binary(vec)).await {
                error!("Error sending message: {}", e);
            }
        }
    }
}

#[derive(Debug)]
pub struct PlayerConnection {
    pub player_name: String,
    pub game: Arc<RwLock<Game>>,
    pub seat: usize,
    pub socket: RwLock<SocketWriter>,
}

async fn receive_client_message<S: Stream<Item = Result<Message, warp::Error>> + Send + Unpin>(
    reader: &mut S,
) -> Result<ClientMessage, Option<String>> {
    loop {
        let ws_msg = match timeout(Duration::from_secs(20), reader.next()).await {
            Ok(Some(Ok(x))) => x,
            // various network errors
            Ok(Some(Err(e))) => return Err(Some(format!("Error receiving message: {}", e))),
            // most likely: connection is already closed
            Ok(None) => return Err(None),
            // timeout
            Err(_) => {
                return Err(Some(
                    "No ping response from client for over 20 seconds".to_owned(),
                ))
            }
        };
        return {
            // this would be cleaner as recursion, but that is messy with async function
            if ws_msg.is_close() {
                Err(None)
            } else if ws_msg.is_ping() || ws_msg.is_pong() {
                // recursion
                continue;
            } else if ws_msg.is_binary() {
                match rmp_serde::from_slice(ws_msg.as_bytes()) {
                    Ok(msg) => Ok(msg),
                    Err(e) => Err(Some(format!("Received corrupted message: {}", e))),
                }
            } else {
                Err(Some("Received corrupted message (unknown type)".to_owned()))
            }
        };
    }
}

impl PlayerConnection {
    pub async fn create_and_start(game_opt: Option<Arc<RwLock<Game>>>, socket: WebSocket) {
        let (w, mut reader) = socket.split();
        let mut writer = SocketWriter(w);
        let Some(game_lock) = game_opt
        else {
            writer.close_with_notice("Game with this ID doesn't exist".to_owned()).await;
            return;
        };

        let (player_name, seat) = match receive_client_message(&mut reader).await {
            Err(err_opt) => {
                if let Some(e) = err_opt {
                    writer.close_with_notice(e).await;
                }
                return;
            }
            Ok(ClientMessage::Init { name, seat }) => (name, seat),
            Ok(_other) => {
                writer
                    .close_with_notice("Unexpected message type (server/client desync)".to_owned())
                    .await;
                return;
            }
        };
        let self_arc = {
            let mut game = game_lock.write().await;
            let map = game.map.clone();
            if let GamePhase::HasWinner(..) = game.phase {
                writer
                    .close_with_notice("This game has already finished".to_owned())
                    .await;
                return;
            }
            let Some(player) = game.players.get_mut(seat)
            else {
                writer.close_with_notice("There aren't that many seats".to_owned()).await;
                return
            };
            if let Some(p) = player.connected.upgrade() {
                writer
                    .close_with_notice(format!(
                        "{} is already connected to this seat",
                        p.player_name
                    ))
                    .await;
                return;
            }
            writer.send_message(ServerMessage::InitInfo(map)).await;

            let conn = Arc::new(Self {
                player_name,
                game: Arc::clone(&game_lock),
                seat,
                socket: RwLock::new(writer),
            });
            player.connected = Arc::downgrade(&conn);
            game.notify_update().await;
            conn
        };
        let self_weak = Arc::downgrade(&self_arc);

        // ping loop
        tokio::spawn(async move {
            while let Some(ping_conn) = self_weak.upgrade() {
                if let Err(e) = ping_conn
                    .socket
                    .write()
                    .await
                    .0
                    .send(Message::ping([]))
                    .await
                {
                    warn!("Error sending ping: {}", e);
                    break;
                }
                // free the Arc, only leave the weak_ref so that the seat is freed as soon as player disconnects
                drop(ping_conn);
                tokio::time::sleep(Duration::from_secs(10)).await;
            }
            debug!("Ending ping loop");
        });

        // reader loop
        tokio::spawn(async move {
            while let Some(msg) = match receive_client_message(&mut reader).await {
                Err(err_opt) => {
                    if let Some(e) = err_opt {
                        self_arc.socket.write().await.close_with_notice(e).await;
                    }
                    None
                }
                Ok(msg) => Some(msg),
            } {
                match msg {
                    ClientMessage::Program(cards) => {
                        let res = self_arc
                            .game
                            .write()
                            .await
                            .program(self_arc.seat, cards)
                            .await;
                        if let Err(e) = res {
                            self_arc
                                .socket
                                .write()
                                .await
                                .send_message(ServerMessage::Notice(e))
                                .await;
                        }
                        let mut game = self_arc.game.write().await;
                        game.notify_update().await;
                        if let GamePhase::Programming(vec) = &game.phase && vec.iter().all(Option::is_some) {
                                drop(game);
                                tokio::spawn(run_moving_phase(Arc::clone(&game_lock)));
                        };
                    }
                    _other => {
                        self_arc
                            .socket
                            .write()
                            .await
                            .close_with_notice(
                                "Unexpected message type (server/client desync)".to_owned(),
                            )
                            .await;
                        break;
                    }
                }
            }
            info!("Ending receive loop for player {}", self_arc.player_name);
            let game = Arc::clone(&self_arc.game);
            drop(self_arc);
            game.write().await.notify_update().await;
        });
    }
}
