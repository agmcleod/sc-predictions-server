use actix::Addr;
use actix_identity::Identity;
use actix_web::{
    web::{block, Data, HttpResponse},
    Result,
};

use auth::{get_claim_from_identity, Role};
use db::{get_conn, models::Round, PgPool};
use errors::Error;

use crate::websocket::{client_messages, Server};

pub async fn lock_round(
    id: Identity,
    websocket_srv: Data<Addr<Server>>,
    pool: Data<PgPool>,
) -> Result<HttpResponse, Error> {
    let (claim, _) = get_claim_from_identity(id)?;
    if claim.role != Role::Owner {
        return Err(Error::Forbidden);
    }

    let conn = get_conn(&pool)?;
    let (conn, claim) = block(move || {
        let round = Round::get_active_round_by_game_id(&conn, claim.game_id)?;
        Round::lock(&conn, round.id)?;
        Ok((conn, claim))
    })
    .await?;

    client_messages::send_game_status(&websocket_srv, conn, claim.game_id).await;
    let conn = get_conn(&pool)?;
    client_messages::send_round_status(&websocket_srv, conn, claim.role, claim.id, claim.game_id)
        .await;

    Ok(HttpResponse::Ok().json(()))
}

#[cfg(test)]
mod tests {
    use actix_web::client::Client;
    use actix_web_actors::ws;
    use diesel::{self, ExpressionMethods, QueryDsl, RunQueryDsl};
    use futures::{SinkExt, StreamExt};
    use serde_json;

    use auth::{create_jwt, PrivateClaim, Role};
    use db::{
        get_conn,
        models::{Game, Round},
        new_pool,
        schema::{games, rounds},
    };
    use errors::ErrorResponse;

    use crate::handlers::{RoundStatusRepsonse, StatusResponse};
    use crate::tests::helpers::tests::{get_test_server, get_websocket_frame_data, test_post};

    #[derive(Insertable)]
    #[table_name = "games"]
    struct NewGame {
        slug: Option<String>,
    }

    #[derive(Insertable)]
    #[table_name = "rounds"]
    pub struct NewRound {
        player_one: String,
        player_two: String,
        game_id: i32,
        locked: bool,
    }

    #[actix_rt::test]
    async fn test_lock_current_round() {
        let pool = new_pool();
        let conn = get_conn(&pool).unwrap();

        let game: Game = diesel::insert_into(games::table)
            .values(NewGame {
                slug: Some("abc123".to_string()),
            })
            .get_result(&conn)
            .unwrap();

        diesel::insert_into(rounds::table)
            .values(vec![
                NewRound {
                    player_one: "maru".to_string(),
                    player_two: "zest".to_string(),
                    game_id: game.id,
                    locked: false,
                },
                NewRound {
                    player_one: "serral".to_string(),
                    player_two: "ty".to_string(),
                    game_id: game.id,
                    locked: true,
                },
            ])
            .execute(&conn)
            .unwrap();

        let slug = game.slug.as_ref().unwrap().clone();
        let claim = PrivateClaim::new(game.id, slug, game.id, Role::Owner);
        let token = create_jwt(claim).unwrap();

        let srv = get_test_server();

        let client = Client::default();
        let mut ws_conn = client.ws(srv.url("/ws/")).connect().await.unwrap();

        // auth this user with the websocket server
        ws_conn
            .1
            .send(ws::Message::Text(format!(
                "/auth {{\"token\":\"{}\"}}",
                token
            )))
            .await
            .unwrap();

        let res = srv
            .post("/api/rounds/lock")
            .set_header("Authorization", token)
            .send()
            .await
            .unwrap();

        assert_eq!(res.status().as_u16(), 200);

        let mut stream = ws_conn.1.take(3);
        // skip the first one, as it's a heartbeat
        stream.next().await;
        let msg = stream.next().await;

        let data = get_websocket_frame_data(msg.unwrap().unwrap());
        if data.is_some() {
            let msg = data.unwrap();
            assert_eq!(msg.path, "/game-status");
            assert_eq!(msg.game_id, game.id);
            let game_status: StatusResponse = serde_json::from_value(msg.data).unwrap();
            // both rounds are locked
            assert_eq!(game_status.open_round, false);
            assert_eq!(game_status.unfinished_round, true);
            assert_eq!(game_status.slug, game.slug.unwrap());
        } else {
            assert!(false, "Message was not a string");
        }

        let msg = stream.next().await;

        let data = get_websocket_frame_data(msg.unwrap().unwrap());
        if data.is_some() {
            let msg = data.unwrap();
            assert_eq!(msg.path, "/round-status");
            assert_eq!(msg.game_id, game.id);
            let round_status: RoundStatusRepsonse = serde_json::from_value(msg.data).unwrap();
            assert_eq!(round_status.locked, true);
        } else {
            assert!(false, "Message was not a string");
        }

        drop(stream);

        srv.stop().await;

        let results: Vec<Round> = rounds::dsl::rounds
            .filter(rounds::dsl::game_id.eq(game.id))
            .get_results(&conn)
            .unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].locked, true);
        assert_eq!(results[1].locked, true);

        diesel::delete(rounds::table).execute(&conn).unwrap();
        diesel::delete(games::table).execute(&conn).unwrap();
    }

    #[actix_rt::test]
    async fn test_lock_current_round_forbidden_for_player() {
        let pool = new_pool();
        let conn = get_conn(&pool).unwrap();

        let game: Game = diesel::insert_into(games::table)
            .values(NewGame {
                slug: Some("abc123".to_string()),
            })
            .get_result(&conn)
            .unwrap();

        diesel::insert_into(rounds::table)
            .values(vec![
                NewRound {
                    player_one: "maru".to_string(),
                    player_two: "zest".to_string(),
                    game_id: game.id,
                    locked: false,
                },
                NewRound {
                    player_one: "serral".to_string(),
                    player_two: "ty".to_string(),
                    game_id: game.id,
                    locked: true,
                },
            ])
            .execute(&conn)
            .unwrap();

        let claim = PrivateClaim::new(game.id, game.slug.unwrap(), game.id, Role::Player);
        let token = create_jwt(claim).unwrap();

        let res: (u16, ErrorResponse) = test_post("/api/rounds/lock", (), Some(token)).await;

        assert_eq!(res.0, 403);

        diesel::delete(rounds::table).execute(&conn).unwrap();
        diesel::delete(games::table).execute(&conn).unwrap();
    }

    #[actix_rt::test]
    async fn test_lock_current_round_no_active_round() {
        let pool = new_pool();
        let conn = get_conn(&pool).unwrap();

        let game: Game = diesel::insert_into(games::table)
            .values(NewGame {
                slug: Some("abc123".to_string()),
            })
            .get_result(&conn)
            .unwrap();

        diesel::insert_into(rounds::table)
            .values(vec![
                NewRound {
                    player_one: "maru".to_string(),
                    player_two: "zest".to_string(),
                    game_id: game.id,
                    locked: true,
                },
                NewRound {
                    player_one: "serral".to_string(),
                    player_two: "ty".to_string(),
                    game_id: game.id,
                    locked: true,
                },
            ])
            .execute(&conn)
            .unwrap();

        let claim = PrivateClaim::new(game.id, game.slug.unwrap(), game.id, Role::Owner);
        let token = create_jwt(claim).unwrap();

        let res: (u16, ErrorResponse) = test_post("/api/rounds/lock", (), Some(token)).await;

        assert_eq!(res.0, 404);

        diesel::delete(rounds::table).execute(&conn).unwrap();
        diesel::delete(games::table).execute(&conn).unwrap();
    }
}
