use std::{sync::Arc, time::Duration};

use futures::{stream::SplitSink, SinkExt, Stream, StreamExt};
use roborally_structs::{
    logging::{error, info, warn},
    transport::{ClientMessage, ServerMessage},
};
use tokio::{
    sync::mpsc::{unbounded_channel, UnboundedSender},
    time::timeout,
};
use warp::ws::{Message, WebSocket};

use crate::game::Game;

#[derive(Debug)]
pub enum SocketMessage {
    CloseWithNotice(String),
    SendMessage(ServerMessage),
    Ping,
}

pub fn create_sender(mut sink: SplitSink<WebSocket, Message>) -> UnboundedSender<SocketMessage> {
    let (sender, mut receiver) = unbounded_channel();
    tokio::task::spawn(async move {
        while let Some(msg) = receiver.recv().await {
            match msg {
                SocketMessage::CloseWithNotice(notice) => {
                    info!("Closing connection with message: {}", &notice);
                    if let Err(e) = sink.send(Message::close_with(1000_u16, notice)).await {
                        warn!("Error when closing connection: {e}");
                    }
                }
                SocketMessage::SendMessage(m) => {
                    if let Err(e) = sink
                        .send(Message::binary(rmp_serde::to_vec(&m).unwrap()))
                        .await
                    {
                        error!("Error sending message: {e}");
                    }
                }
                SocketMessage::Ping => sink.send(Message::ping(Vec::new())).await.unwrap(),
            }
        }
    });
    sender
}

pub struct PlayerConnection {
    pub player_name: String,
    pub game: Arc<Game>,
    pub seat: usize,
    pub sender: UnboundedSender<SocketMessage>,
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
            Ok(Some(Err(e))) => return Err(Some(format!("Error receiving message: {e}"))),
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
                    Err(e) => Err(Some(format!("Received corrupted message: {e}"))),
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
        game_opt: Option<Arc<Game>>,
        socket: WebSocket,
        player_name: String,
        seat: usize,
    ) {
        use SocketMessage::*;
        let (w, mut reader) = socket.split();
        let sender = create_sender(w);
        let Some(game) = game_opt
        else {
            sender.send(CloseWithNotice("Game with this ID doesn't exist".to_owned())).unwrap();
            return;
        };

        let self_arc = {
            let mut state = game.state.write().unwrap();
            let Some(player) = state.players.get_mut(seat)
            else {
                drop(state);
                sender.send(CloseWithNotice("There aren't that many seats".to_owned())).unwrap();
                return;
            };
            if let Some(p) = player.connected.upgrade() {
                drop(state);
                sender
                    .send(CloseWithNotice(format!(
                        "{} is already connected to this seat",
                        p.player_name
                    )))
                    .unwrap();
                return;
            }

            let conn = Arc::new(Self {
                player_name,
                game: Arc::clone(&game),
                seat,
                sender,
            });
            player.connected = Arc::downgrade(&conn);
            state.send_programming_state_to_player(seat);
            state.send_general_state();
            conn
        };

        let self_weak = Arc::downgrade(&self_arc);
        // ping loop
        tokio::spawn(async move {
            while let Some(ping_conn) = self_weak.upgrade() {
                if let Err(e) = ping_conn.sender.send(Ping) {
                    warn!("Error sending ping: {e}");
                    break;
                }
                // free the Arc, only leave the Weak so that the seat is freed as soon as player disconnects
                drop(ping_conn);
                tokio::time::sleep(Duration::from_secs(10)).await;
            }
        });

        // reader loop
        tokio::spawn(async move {
            while let Some(msg) = match receive_client_message(&mut reader).await {
                Err(err_opt) => {
                    if let Some(e) = err_opt {
                        self_arc.sender.send(CloseWithNotice(e)).unwrap();
                    }
                    None
                }
                Ok(msg) => Some(msg),
            } {
                match msg {
                    ClientMessage::Program(cards) => {
                        let res = self_arc.game.program(self_arc.seat, cards).await;
                        if let Err(e) = res {
                            self_arc
                                .sender
                                .send(SocketMessage::SendMessage(ServerMessage::Notice(e)))
                                .unwrap();
                        }
                    }
                }
            }
            info!("Ending receive loop for player {}", self_arc.player_name);
            let game_arc = Arc::clone(&self_arc.game);
            drop(self_arc);
            game_arc.state.read().unwrap().send_general_state();
        });
    }
}
