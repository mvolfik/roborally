use std::{sync::Arc, time::Duration};

use futures::{stream::SplitSink, SinkExt, Stream, StreamExt};
use roborally_structs::{
    logging::{error, info, warn},
    transport::{ClientMessage, ServerMessage},
};
use tokio::{sync::RwLock, time::timeout};
use warp::ws::{Message, WebSocket};

use crate::game::{Game, GamePhase};

#[derive(Debug)]
pub struct SocketWriter(SplitSink<WebSocket, Message>);

impl SocketWriter {
    pub async fn close_with_notice(&mut self, msg: String) {
        info!("Closing connection with message: {}", &msg);
        if let Err(e) = self.0.send(Message::close_with(1000_u16, msg)).await {
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

/// Attempts to receive a message
///
/// If `Err(Some(String))` is returned, the associated writer should be closed with that message.
///
/// If `Err(None)` is returned, the writer is already closed.
///
/// In either `Err` case, this function shouldn't be called again for the same reader
async fn receive_client_message<S: Stream<Item = Result<Message, warp::Error>> + Send + Unpin>(
    reader: &mut S,
) -> Result<ClientMessage, Option<String>> {
    // this function would be cleaner using recursion, but with async function that requires boxing and can cause lifetime checker issues
    loop {
        // even if the player doesn't make any action for 20 seconds, at least a `pong` should be received
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
    /// Creates a player connections and starts receive loop
    ///
    /// The connection isn't returned - it lives in an `Arc` (reference-counted pointer), which is dropped when the receive loop ends
    ///
    /// Weak references to the `Arc` are stored in the game object, and in a ping keepalive loop
    pub async fn create_and_start(
        game_opt: Option<Arc<RwLock<Game>>>,
        socket: WebSocket,
        player_name: String,
        seat: usize,
    ) {
        let (w, mut reader) = socket.split();
        let mut writer = SocketWriter(w);
        let Some(game_lock) = game_opt
        else {
            writer.close_with_notice("Game with this ID doesn't exist".to_owned()).await;
            return;
        };

        let self_arc = {
            let mut game = game_lock.write().await;
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

            let conn = Arc::new(Self {
                player_name,
                game: Arc::clone(&game_lock),
                seat,
                socket: RwLock::new(writer),
            });
            player.connected = Arc::downgrade(&conn);
            game.send_single_state().await;
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
                    }
                }
            }
            info!("Ending receive loop for player {}", self_arc.player_name);
            let game = Arc::clone(&self_arc.game);
            drop(self_arc);
            game.write().await.send_single_state().await;
        });
    }
}
