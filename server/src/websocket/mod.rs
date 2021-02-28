use std::time::{Duration, Instant};

use actix::{
    fut,
    prelude::{Actor, Addr, StreamHandler},
    ActorContext, ActorFuture, AsyncContext, ContextFutureSpawner, Handler, WrapFuture,
};
use actix_web::{web, HttpRequest, HttpResponse};
use actix_web_actors::ws;
use serde::Deserialize;
use serde_json;
use uuid::Uuid;

use errors::Error;

pub mod client_messages;
mod server;

pub use self::server::*;

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);
const CLIENT_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Deserialize)]
struct AuthReq {
    token: String,
}

pub struct WebSocketSession {
    id: String,
    hb: Instant,
    token: Option<String>,
    server_addr: Addr<Server>,
}

impl WebSocketSession {
    fn new(server_addr: Addr<Server>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            hb: Instant::now(),
            token: None,
            server_addr,
        }
    }

    fn send_heartbeat(&self, ctx: &mut <Self as Actor>::Context) {
        ctx.run_interval(HEARTBEAT_INTERVAL, |act, ctx| {
            if Instant::now().duration_since(act.hb) > CLIENT_TIMEOUT {
                info!("Websocket Client heartbeat failed, disconnecting!");
                act.server_addr.do_send(Disconnect { id: act.id.clone() });
                // stop actor
                ctx.stop();

                // don't try to send a ping
                return;
            }
            ctx.ping(b"");
        });
    }
}

impl Actor for WebSocketSession {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.send_heartbeat(ctx);

        let session_addr = ctx.address();
        self.server_addr
            .send(Connect {
                addr: session_addr.recipient(),
                id: self.id.clone(),
            })
            .into_actor(self)
            .then(|res, _act, ctx| {
                match res {
                    Ok(_res) => {}
                    _ => ctx.stop(),
                }
                fut::ready(())
            })
            .wait(ctx);
    }
}

impl Handler<Message> for WebSocketSession {
    type Result = ();

    fn handle(&mut self, msg: Message, ctx: &mut Self::Context) {
        ctx.text(msg.0);
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for WebSocketSession {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Ping(msg)) => {
                self.hb = Instant::now();
                ctx.pong(&msg);
            }
            Ok(ws::Message::Pong(_)) => {
                self.hb = Instant::now();
            }
            Ok(ws::Message::Text(text)) => {
                let message = text.trim();
                // has routing pattern
                if message.starts_with("/") {
                    let args: Vec<&str> = message.splitn(2, ' ').collect();
                    match args[0] {
                        "/auth" => {
                            let params: Result<AuthReq, serde_json::Error> =
                                serde_json::from_str(args[1]);
                            if let Ok(params) = params {
                                self.server_addr.do_send(Auth {
                                    id: self.id.clone(),
                                    token: params.token,
                                });
                            } else {
                                ctx.text("Invalid request params");
                            }
                        }
                        _ => ctx.text(format!("unknown command {:?}", message)),
                    }
                }
            }
            Ok(ws::Message::Binary(bin)) => ctx.binary(bin),
            Ok(ws::Message::Close(reason)) => {
                info!("closed ws session");
                self.server_addr.do_send(Disconnect {
                    id: self.id.clone(),
                });
                ctx.close(reason);
                ctx.stop();
            }
            Err(err) => {
                warn!("Error handling msg: {:?}", err);
                ctx.stop()
            }
            _ => ctx.stop(),
        }
    }
}

pub async fn ws_index(
    req: HttpRequest,
    stream: web::Payload,
    server_addr: web::Data<Addr<Server>>,
) -> Result<HttpResponse, Error> {
    let res = ws::start(
        WebSocketSession::new(server_addr.get_ref().clone()),
        &req,
        stream,
    )?;

    Ok(res)
}
