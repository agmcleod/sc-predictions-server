use std::time::{Duration, Instant};

use actix::{ActorContext, AsyncContext, fut, Handler, prelude::{Actor, Addr, StreamHandler}};
use actix_web::{web, HttpRequest, HttpResponse};
use actix_web_actors::ws;
use serde::Deserialize;
use serde_json;
use uuid::Uuid;

use errors::Error;

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
        Self { id: Uuid::new_v4().to_string(), hb: Instant::now(), token: None, server_addr }
    }

    fn send_heartbeat(&self, ctx: &mut <Self as Actor>::Context) {
        ctx.run_interval(HEARTBEAT_INTERVAL, |act, ctx| {
            if Instant::now().duration_since(act.hb) > CLIENT_TIMEOUT {
                println!("Websocket Client heartbeat failed, disconnecting!");

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

        let addr = ctx.address();
        addr.send(Connect {
            addr: addr.recipient(),
            id: self.id.clone(),
        })
        .into_actor(self)
        .then(|res, act, ctx| {
            match res {
                Ok(res) => act.id = res,
                _ => ctx.stop(),
            }
            fut::ready(())
        })
        .wait(ctx);
    }
}

impl Handler<Message> for WebSocketSession {
    type Result = ();

    fn handle(&mut self, msg: server::Message, ctx: &mut Self::Context) {
        ctx.text(format!("{}, {}", msg.path, msg.data));
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for WebSocketSession {
    fn handle(
        &mut self,
        msg: Result<ws::Message, ws::ProtocolError>,
        ctx: &mut Self::Context,
    ) {
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
                            let params: Result<AuthReq, serde_json::Error> = serde_json::from_str(args[1]);
                            if let Ok(params) = params {
                                self.server_addr.do_send(msg)
                            } else {
                                ctx.text("Invalid request params");
                            }
                        },
                        _ => ctx.text(format!("unknown command {:?}", message))
                    }
                }
            },
            Ok(ws::Message::Binary(bin)) => ctx.binary(bin),
            Ok(ws::Message::Close(reason)) => {
                ctx.close(reason);
                ctx.stop();
            }
            _ => ctx.stop(),
        }
    }
}

pub async fn ws_index(req: HttpRequest, stream: web::Payload, server_addr: web::Data<Addr<Server>>) -> Result<HttpResponse, Error> {
    let res = ws::start(WebSocketSession::new(server_addr.get_ref().clone()), &req, stream)?;

    Ok(res)
}