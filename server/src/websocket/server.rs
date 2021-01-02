use std::collections::HashMap;

use actix::prelude::{Actor, Context, Handler, Message as ActixMessage, Recipient};
use serde::Serialize;
use serde_json::{to_string, Value};

use auth::{decode_jwt, PrivateClaim};
use db::PgPool;

#[derive(ActixMessage)]
#[rtype(result = "()")]
pub struct Message(pub String);

#[derive(ActixMessage, Serialize)]
#[rtype(result = "()")]
pub struct MessageToClient {
    pub path: String,
    pub data: Value,
    pub game_id: i32,
}

impl MessageToClient {
    pub fn new(path: &str, game_id: i32, data: Value) -> MessageToClient {
        Self {
            path: path.to_string(),
            data,
            game_id,
        }
    }
}

struct Session {
    addr: Recipient<Message>,
    token: Option<String>,
}

impl Session {
    fn new(addr: Recipient<Message>) -> Self {
        Session { addr, token: None }
    }
}

pub struct Server {
    game_to_sessions: HashMap<i32, Vec<String>>,
    pool: PgPool,
    sessions: HashMap<String, Session>,
}

impl Server {
    pub fn new(pool: PgPool) -> Self {
        Server {
            game_to_sessions: HashMap::new(),
            pool,
            sessions: HashMap::new(),
        }
    }
}

impl Actor for Server {
    type Context = Context<Self>;
}

#[derive(ActixMessage)]
#[rtype(result = "()")]
pub struct Auth {
    pub id: String,
    pub token: String,
}

impl Handler<Auth> for Server {
    type Result = ();

    fn handle(&mut self, msg: Auth, _: &mut Context<Self>) -> Self::Result {
        let private_claim = decode_jwt(&msg.token);
        if private_claim.is_ok() {
            if !self.sessions.contains_key(&msg.id) {
                error!("Session not found: {}", msg.id);
                return;
            }
            self.sessions.get_mut(&msg.id).unwrap().token = Some(msg.token.clone());
            let private_claim = private_claim.unwrap();
            if !self.game_to_sessions.contains_key(&private_claim.game_id) {
                self.game_to_sessions
                    .insert(private_claim.game_id, Vec::new());
            }

            let sessions_for_game = self
                .game_to_sessions
                .get_mut(&private_claim.game_id)
                .unwrap();
            sessions_for_game.push(msg.id.clone());
        }
    }
}

#[derive(ActixMessage)]
#[rtype(result = "()")]
pub struct Connect {
    pub addr: Recipient<Message>,
    pub id: String,
}

impl Handler<Connect> for Server {
    type Result = ();

    fn handle(&mut self, msg: Connect, _: &mut Context<Self>) {
        self.sessions.insert(msg.id.clone(), Session::new(msg.addr));
    }
}

impl Handler<MessageToClient> for Server {
    type Result = ();

    fn handle(&mut self, msg: MessageToClient, _: &mut Context<Self>) -> Self::Result {
        if let Some(session_ids) = self.game_to_sessions.get(&msg.game_id) {
            for id in session_ids {
                if let Some(session) = self.sessions.get(id) {
                    if let Ok(data) = to_string(&msg) {
                        match session.addr.do_send(Message(data)) {
                            Err(err) => {
                                error!("Error sending client message: {:?}", err);
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    }
}
