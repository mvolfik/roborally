use std::sync::Arc;

use futures::{stream::SplitSink, SinkExt, StreamExt};
use roborally_structs::{
    logging::{error, info, warn},
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
        self.0.send(Message::close()).await.unwrap();
    }

    pub async fn send_message(&mut self, msg: ServerMessage) {
        if let Ok(vec) = rmp_serde::to_vec(&msg) {
            self.0
                .send(Message::binary(vec))
                .await
                .unwrap_or_else(|e| error!("{}", e));
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

impl PlayerConnection {
    pub async fn create_and_start(game_opt: Option<Arc<RwLock<Game>>>, socket: WebSocket) {
        let (w, mut reader) = socket.split();
        let mut writer = SocketWriter(w);
        let Some(game_lock) = game_opt else {
            writer.close_with_notice("Game with this ID doesn't exist".to_string()).await;
            return;
        };

        let (player_name, seat) = match reader.next().await {
            None => {
                error!("unexpected None");
                return;
            }
            Some(Err(e)) => {
                writer
                    .close_with_notice(format!("Connection error: {}", e))
                    .await;
                return;
            }
            Some(Ok(ws_msg)) if ws_msg.is_close() => {
                writer.0.close().await.unwrap();
                return;
            }
            Some(Ok(ws_msg)) => match rmp_serde::from_slice::<ClientMessage>(ws_msg.as_bytes()) {
                Ok(ClientMessage::Init { name, seat }) => (name, seat),
                Ok(_other) => {
                    writer
                        .close_with_notice(
                            "Unexpected message type (server/client desync)".to_string(),
                        )
                        .await;
                    return;
                }
                Err(e) => {
                    writer
                        .close_with_notice(format!("Corrupted message: {}", e))
                        .await;
                    return;
                }
            },
        };
        let self_arc = {
            let mut game = game_lock.write().await;
            let map = game.map.clone();
            let Some(player) = game.players.get_mut(seat) else {
                writer.close_with_notice("There aren't that many seats".to_string()).await;
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
                game: game_lock.clone(),
                seat,
                socket: RwLock::new(writer),
            });
            player.connected = Arc::downgrade(&conn);
            game.notify_update().await;
            conn
        };
        tokio::task::spawn(async move {
            while let Some(ws_res) = reader.next().await {
                let res: Result<(), String> = match ws_res {
                    Err(e) => Err(format!("Connection error: {}", e)),
                    Ok(ws_msg) if ws_msg.is_close() => {
                        self_arc.socket.write().await.0.close().await.unwrap();
                        Ok(())
                    }
                    Ok(ws_msg) => match rmp_serde::from_slice::<ClientMessage>(ws_msg.as_bytes()) {
                        Ok(ClientMessage::Program(cards)) => {
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
                            let game = self_arc.game.read().await;
                            game.notify_update().await;
                            if let GamePhase::Programming(vec) = &game.phase && vec.iter().all(Option::is_some) {
                                tokio::spawn(run_moving_phase(game_lock.clone()));
                            }
                            Ok(())
                        }
                        Ok(_other) => {
                            Err("Unexpected message type (server/client desync)".to_string())
                        }
                        Err(e) => Err(format!("Corrupted message: {}", e)),
                    },
                };
                if let Err(e) = res {
                    self_arc.socket.write().await.close_with_notice(e).await;
                }
            }
            info!("Ending receive loop for player {}", self_arc.name);
        });
    }
}
