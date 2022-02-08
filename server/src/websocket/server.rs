use std::collections::HashMap;

use actix::prelude::{Actor, Context, Handler, Message as ActixMessage, Recipient};
use serde::{Deserialize, Serialize};
use serde_json::{error::Result as SerdeResult, to_string, to_value, Value};

use auth::decode_jwt;
use db::{get_conn, models::User, PgPool};
use errors::Error;

#[derive(ActixMessage)]
#[rtype(result = "()")]
pub struct Message(pub String);

#[derive(ActixMessage, Deserialize, Serialize)]
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
    // should only be one, but lets track multiple in case
    game_ids: Vec<i32>,
}

impl Session {
    fn new(addr: Recipient<Message>) -> Self {
        Session {
            addr,
            token: None,
            game_ids: Vec::new(),
        }
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
            let current_session = self.sessions.get_mut(&msg.id).unwrap();
            current_session.token = Some(msg.token.clone());
            let private_claim = private_claim.unwrap();
            if !self.game_to_sessions.contains_key(&private_claim.game_id) {
                self.game_to_sessions
                    .insert(private_claim.game_id, Vec::new());
            }

            let sessions_for_game = self
                .game_to_sessions
                .get_mut(&private_claim.game_id)
                .unwrap();

            // already authenticated
            if sessions_for_game.contains(&msg.id) {
                return Ok(());
            }
            sessions_for_game.push(msg.id.clone());

            current_session.game_ids.push(private_claim.game_id);

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

#[cfg(test)]
mod tests {
    use actix_web_actors::ws;
    use awc::Client;
    use diesel::RunQueryDsl;
    use futures::{SinkExt, StreamExt};
    use serde_json;

    use auth::{PrivateClaim, Role};
    use db::{
        get_conn,
        models::{Game, NewUser, User, UserDetails},
        new_pool,
        schema::{games, users},
    };

    use crate::tests::helpers::tests::{get_auth_token, get_test_server, get_websocket_frame_data};

    #[derive(Insertable)]
    #[table_name = "games"]
    struct NewGame {
        slug: String,
    }

    #[actix_rt::test]
    async fn test_ws_auth_broadcast_no_users() {
        let pool = new_pool();
        let conn = get_conn(&pool).unwrap();

        let game: Game = diesel::insert_into(games::table)
            .values(NewGame {
                slug: "abc123".to_string(),
            })
            .get_result(&conn)
            .unwrap();

        let srv = get_test_server();

        let client = Client::default();
        let mut ws_conn = client.ws(srv.url("/ws/")).connect().await.unwrap();

        // owner joins
        let token = get_auth_token(PrivateClaim::new(
            game.id,
            "player one".to_string(),
            game.id,
            Role::Owner,
        ));

        ws_conn
            .1
            .send(ws::Message::Text(
                format!("/auth {{\"token\":\"{}\"}}", token).into(),
            ))
            .await
            .unwrap();

        let mut stream = ws_conn.1.take(1);

        let msg = stream.next().await;
        let data = get_websocket_frame_data(msg.unwrap().unwrap());
        if data.is_some() {
            let msg = data.unwrap();
            assert_eq!(msg.path, "/players");
            assert_eq!(msg.game_id, game.id);
            let players = msg.data.as_array().unwrap();
            assert_eq!(players.len(), 0);
        } else {
            assert!(false, "Message was not a string");
        }

        drop(stream);

        srv.stop().await;
        diesel::delete(games::table).execute(&conn).unwrap();
    }

    #[actix_rt::test]
    async fn test_ws_auth_broadcasts_users() {
        let pool = new_pool();
        let conn = get_conn(&pool).unwrap();

        let game: Game = diesel::insert_into(games::table)
            .values(NewGame {
                slug: "abc123".to_string(),
            })
            .get_result(&conn)
            .unwrap();

        let srv = get_test_server();

        let client = Client::default();
        let mut ws_conn = client.ws(srv.url("/ws/")).connect().await.unwrap();

        let user: User = diesel::insert_into(users::table)
            .values(NewUser {
                game_id: game.id,
                user_name: "agmcleod".to_string(),
            })
            .get_result(&conn)
            .unwrap();

        // player joins
        let token = get_auth_token(PrivateClaim::new(
            user.id,
            "player one".to_string(),
            game.id,
            Role::Player,
        ));

        ws_conn
            .1
            .send(ws::Message::Text(
                format!("/auth {{\"token\":\"{}\"}}", token).into(),
            ))
            .await
            .unwrap();

        let mut stream = ws_conn.1.take(1);

        let msg = stream.next().await;
        let data = get_websocket_frame_data(msg.unwrap().unwrap());
        if data.is_some() {
            let msg = data.unwrap();
            assert_eq!(msg.path, "/players");
            assert_eq!(msg.game_id, game.id);
            let players = msg.data.as_array().unwrap();
            assert_eq!(players.len(), 1);
            let player = players.get(0).unwrap().to_owned();
            let player: UserDetails = serde_json::from_value(player).unwrap();
            assert_eq!(player.user_name, "agmcleod");
            assert_eq!(player.game_id, game.id);
        } else {
            assert!(false, "Message was not a string");
        }

        drop(stream);

        srv.stop().await;
        diesel::delete(users::table).execute(&conn).unwrap();
        diesel::delete(games::table).execute(&conn).unwrap();
    }
}
