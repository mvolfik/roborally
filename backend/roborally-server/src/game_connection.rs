use std::time::Duration;

use actix::fut::wrap_future;
use actix::{Actor, ActorFutureExt, Addr, Handler, Message, StreamHandler};
use actix::{ActorTryFutureExt, AsyncContext};
use actix_web_actors::ws;
use futures::Future;
use roborally_structs::transport::{ClientMessage, ServerMessage};

use crate::game::{Disconnect, Game, Program, RequestConnect};

pub struct GameConnection {
    pub game: Addr<Game>,
    /// If none - the connection isn't yet fully initialized
    pub seat: Option<usize>,
}

impl Actor for GameConnection {
    type Context = ws::WebsocketContext<Self>;

    fn stopping(&mut self, ctx: &mut Self::Context) -> actix::Running {
        if let Some(seat) = self.seat {
            self.game.do_send(Disconnect(seat));
        }
        actix::Running::Stop
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for GameConnection {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        let bytes = match msg {
            Ok(ws::Message::Ping(msg)) => {
                ctx.pong(&msg);
                return;
            }
            Ok(ws::Message::Pong(_)) => return,
            Ok(ws::Message::Close(_)) => {
                ctx.close(None);
                return;
            }
            Ok(ws::Message::Binary(b)) => b,
            x => {
                ctx.notify(CloseConnection(format!(
                    "Received unknown message type: {:?}",
                    x
                )));
                return;
            }
        };
        let Ok(data) = rmp_serde::from_slice::<ClientMessage>(&bytes) else {
            ctx.notify(CloseConnection("Corrupted message".to_string()));
            return;
        };

        match (self.seat, data) {
            (None, ClientMessage::Init { name, seat }) => {
                self.game.do_send(RequestConnect {
                    name,
                    seat,
                    weak_addr: ctx.address().downgrade(),
                });
                self.seat = Some(seat);
            }
            (Some(seat), ClientMessage::Program(cards)) => {
                self.game.do_send(Program(seat, cards));
            }
            x => ctx.notify(CloseConnection(format!(
                "Server and client got out of sync: {:?}",
                x
            ))),
        }
    }

    fn started(&mut self, ctx: &mut Self::Context) {
        ctx.run_interval(Duration::from_secs(30), |_conn, ctx| {
            ctx.ping(&[]);
        });
    }
}

pub struct ServerActorMessage(pub ServerMessage);
impl Message for ServerActorMessage {
    type Result = ();
}

impl Handler<ServerActorMessage> for GameConnection {
    type Result = ();

    fn handle(&mut self, msg: ServerActorMessage, ctx: &mut Self::Context) -> Self::Result {
        ctx.binary(rmp_serde::to_vec(&msg.0).unwrap());
    }
}

pub struct CloseConnection(pub String);
impl Message for CloseConnection {
    type Result = ();
}
impl Handler<CloseConnection> for GameConnection {
    type Result = ();

    fn handle(&mut self, msg: CloseConnection, ctx: &mut Self::Context) -> Self::Result {
        println!("Disconnecting player: {}", &msg.0);
        let addr = ctx.address();
        ctx.spawn(
            wrap_future::<_, Self>(addr.send(ServerActorMessage(ServerMessage::Notice(msg.0))))
                .map(|res, _, ctx| ctx.close(None)),
        );
    }
}
