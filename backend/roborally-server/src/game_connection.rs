use std::{sync::Arc, time::Duration};

use futures::{
    stream::{SplitSink, SplitStream},
    SinkExt, StreamExt,
};
use roborally_structs::{
    logging::{debug, error, info, warn},
    transport::{ClientMessage, ServerMessage},
};
use tokio::sync::RwLock;
use warp::ws::{Message, WebSocket};

use crate::game::{run_moving_phase, Game, GamePhase};

#[derive(Debug)]
pub struct SocketWriter(SplitSink<WebSocket, Message>);

impl SocketWriter {
    pub async fn close_with_notice(&mut self, msg: String) {
        warn!("Closing connection with message: {}", &msg);
        self.send_message(ServerMessage::Notice(msg)).await;
        self.0
            .send(Message::close())
            .await
            .unwrap_or_else(|e| warn!("Error when closing connection: {}", e));
    }

    pub async fn send_message(&mut self, msg: ServerMessage) {
        if let Ok(vec) = rmp_serde::to_vec(&msg) {
            self.0
                .send(Message::binary(vec))
                .await
                .unwrap_or_else(|e| error!("Error sending message: {}", e));
        }
    }
}

#[derive(Debug)]
pub struct PlayerConnection {
    pub name: String,
    pub game: Arc<RwLock<Game>>,
    pub seat: usize,
    pub socket: RwLock<SocketWriter>,
}

async fn receive_client_message(
    reader: &mut SplitStream<WebSocket>,
) -> Result<ClientMessage, Option<String>> {
    loop {
        // if None, the connection is already closed. In all other cases, do close_with_notice and return None
        let ws_msg = match reader.next().await.ok_or(None)? {
            Ok(x) => x,
            Err(e) => break Err(Some(format!("Error receiving message: {}", e))),
        };
        break (
            // this would be cleaner as recursion, but that is messy with async function
            if ws_msg.is_close() {
                Err(None)
            } else if ws_msg.is_pong() || ws_msg.is_ping() {
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
        );
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
                    .close_with_notice(format!("{} is already connected to this seat", p.name))
                    .await;
                return;
            }
            writer.send_message(ServerMessage::InitInfo(map)).await;

            let conn = Arc::new(Self {
                name: player_name,
                game: Arc::clone(&game_lock),
                seat,
                socket: RwLock::new(writer),
            });
            player.connected = Arc::downgrade(&conn);
            game.notify_update().await;
            conn
        };
        let self_weak = Arc::downgrade(&self_arc);
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
                tokio::time::sleep(Duration::from_secs(15)).await;
            }
            debug!("Ending ping loop");
        });
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
            info!("Ending receive loop for player {}", self_arc.name);
        });
    }
}
