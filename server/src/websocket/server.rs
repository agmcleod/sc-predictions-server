use std::collections::HashMap;

use actix::prelude::{Actor, Context, Handler, Message as ActixMessage, Recipient};
use serde::Serialize;
use serde_json::{error::Result as SerdeResult, to_string, to_value, Value};

use auth::{decode_jwt, PrivateClaim};
use db::{get_conn, models::User, PgPool};
use errors::Error;

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

    fn send_msg_to_game_sessions(&self, game_id: &i32, data: SerdeResult<String>) {
        if let Some(session_ids) = self.game_to_sessions.get(game_id) {
            for id in session_ids {
                if let Some(session) = self.sessions.get(id) {
                    if let Ok(ref data) = data {
                        match session.addr.do_send(Message(data.clone())) {
                            Err(err) => {
                                error!("Error sending client message: {:?}", err);
                            }
                            _ => {}
                        }
                    }
                }
            }
        } else {
            warn!("Could not find session by game: {}", *game_id);
        }
    }
}

impl Actor for Server {
    type Context = Context<Self>;
}

pub struct Auth {
    pub id: String,
    pub token: String,
}

impl ActixMessage for Auth {
    type Result = Result<(), Error>;
}

impl Handler<Auth> for Server {
    type Result = Result<(), Error>;

    fn handle(&mut self, msg: Auth, _: &mut Context<Self>) -> Self::Result {
        let private_claim = decode_jwt(&msg.token);
        if private_claim.is_ok() {
            if !self.sessions.contains_key(&msg.id) {
                error!("Session not found: {}", msg.id);
                return Ok(());
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

            let connection = get_conn(&self.pool)?;

            let users = User::find_all_by_game_id(&connection, private_claim.game_id)?;
            if let Ok(value) = to_value(users) {
                let msg = MessageToClient::new("/players", private_claim.game_id, value);
                self.send_msg_to_game_sessions(&msg.game_id, to_string(&msg));
            }
        }

        Ok(())
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

#[derive(ActixMessage)]
#[rtype(result = "()")]
pub struct Disconnect {
    pub id: String,
}

impl Handler<Disconnect> for Server {
    type Result = ();

    fn handle(&mut self, msg: Disconnect, _: &mut Context<Self>) {
        self.sessions.remove(&msg.id);
    }
}

impl Handler<MessageToClient> for Server {
    type Result = ();

    fn handle(&mut self, msg: MessageToClient, _: &mut Context<Self>) -> Self::Result {
        self.send_msg_to_game_sessions(&msg.game_id, to_string(&msg));
    }
}
