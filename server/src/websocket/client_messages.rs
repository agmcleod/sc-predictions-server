use actix::Addr;
use actix_web::web::Data;
use diesel::pg::PgConnection;
use diesel::r2d2::{ConnectionManager, PooledConnection};
use serde_json::to_value;

use super::{MessageToClient, Server};
use crate::handlers;

pub async fn send_game_status(
    websocket_srv: &Data<Addr<Server>>,
    connection: PooledConnection<ConnectionManager<PgConnection>>,
    game_id: i32,
) {
    let status_response = handlers::get_game_status(connection, game_id).await;
    match status_response {
        Ok(status_response) => {
            if let Ok(value) = to_value(status_response) {
                let msg = MessageToClient::new("/game-status", game_id, value);
                websocket_srv.do_send(msg);
            }
        }
        Err(err) => error!("{:?}", err),
    }
}

pub async fn send_round_status(
    websocket_srv: &Data<Addr<Server>>,
    connection: PooledConnection<ConnectionManager<PgConnection>>,
    game_id: i32,
) {
    let round_status = handlers::get_round_status(connection, game_id).await;
    match round_status {
        Ok(round_status) => {
            if let Ok(value) = to_value(round_status) {
                let msg = MessageToClient::new("/round-status", game_id, value);
                websocket_srv.do_send(msg);
            }
        }
        Err(err) => error!("{:?}", err),
    }
}
