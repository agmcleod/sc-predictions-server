use actix::Addr;
use actix_identity::Identity;
use actix_web::{
    web::{block, Data, Json},
    Result,
};
use serde::{Deserialize, Serialize};
use validator::Validate;

use auth::{get_claim_from_identity, Role};
use db::{
    get_conn,
    models::{Game, Round},
    PgPool,
};
use errors::Error;

use crate::validate::validate;
use crate::websocket::{client_messages, Server};

#[derive(Clone, Deserialize, Serialize, Validate)]
pub struct CreateRoundRequest {
    #[validate(length(min = "1"))]
    player_one: String,
    #[validate(length(min = "1"))]
    player_two: String,
}

pub async fn create(
    id: Identity,
    websocket_srv: Data<Addr<Server>>,
    pool: Data<PgPool>,
    params: Json<CreateRoundRequest>,
) -> Result<Json<Round>, Error> {
    validate(&params)?;

    let (claim, token) = get_claim_from_identity(id)?;
    if claim.role != Role::Owner {
        return Err(Error::Forbidden);
    }

    let conn = get_conn(&pool)?;

    let game_id = claim.game_id;

    let res = block(move || {
        Game::find_by_id(&conn, game_id).map_err(|err| match err {
            // if the game didnt exist, return a forbidden error
            Error::NotFound(_) => Error::Forbidden,
            _ => err,
        })
    })
    .await?;

    let game = res?;

    if game.creator.is_none() || game.creator.unwrap() != token {
        return Err(Error::Forbidden);
    }

    let conn = get_conn(&pool)?;
    let res = block(move || {
        Round::create(
            &conn,
            game_id,
            params.player_one.clone(),
            params.player_two.clone(),
        )
    })
    .await?;

    let round = res?;

    let conn = get_conn(&pool)?;
    client_messages::send_game_status(&websocket_srv, conn, claim.game_id).await;

    let conn = get_conn(&pool)?;
    client_messages::send_round_status(&websocket_srv, conn, claim.role, claim.id, claim.game_id)
        .await;

    Ok(Json(round))
}

#[cfg(test)]
mod tests {
    use actix_web_actors::ws;
    use awc::Client;
    use diesel::{self, ExpressionMethods, QueryDsl, RunQueryDsl};
    use futures::{SinkExt, StreamExt};

    use auth::{create_jwt, PrivateClaim, Role};
    use db::{
        get_conn,
        models::{Game, Round},
        new_pool,
        schema::{games, rounds},
    };
    use errors::ErrorResponse;

    use super::CreateRoundRequest;
    use crate::handlers::StatusResponse;
    use crate::tests::helpers::tests::{get_test_server, get_websocket_frame_data, test_post};

    #[derive(Insertable)]
    #[table_name = "games"]
    struct NewGame {
        slug: Option<String>,
    }

    #[actix_rt::test]
    async fn test_create_round_as_owner() {
        let pool = new_pool();
        let conn = get_conn(&pool).unwrap();

        let game: Game = diesel::insert_into(games::table)
            .values(NewGame {
                slug: Some("abc123".to_string()),
            })
            .get_result(&conn)
            .unwrap();
        let claim = PrivateClaim::new(
            game.id,
            game.slug.as_ref().unwrap().to_owned(),
            game.id,
            Role::Owner,
        );
        let token = create_jwt(claim).unwrap();

        diesel::update(games::dsl::games.find(game.id))
            .set(games::dsl::creator.eq(token.clone()))
            .execute(&conn)
            .unwrap();

        let srv = get_test_server();

        let client = Client::default();
        let mut ws_conn = client.ws(srv.url("/ws/")).connect().await.unwrap();

        ws_conn
            .1
            .send(ws::Message::Text(
                format!("/auth {{\"token\":\"{}\"}}", token).into(),
            ))
            .await
            .unwrap();

        let mut res = srv
            .post("/api/rounds")
            .append_header(("Authorization", token))
            .send_json(&CreateRoundRequest {
                player_one: "Boxer".to_string(),
                player_two: "Idra".to_string(),
            })
            .await
            .unwrap();

        assert_eq!(res.status().as_u16(), 200);

        let round: Round = res.json().await.unwrap();

        assert_eq!(round.player_one, "Boxer");
        assert_eq!(round.player_two, "Idra");

        let mut stream = ws_conn.1.take(2);
        // skip the first one, as it's a heartbeat
        stream.next().await;
        let msg = stream.next().await;

        let data = get_websocket_frame_data(msg.unwrap().unwrap());
        if data.is_some() {
            let msg = data.unwrap();
            assert_eq!(msg.path, "/game-status");
            assert_eq!(msg.game_id, game.id);
            let game_status: StatusResponse = serde_json::from_value(msg.data).unwrap();
            // round unlocked & unfinished
            assert_eq!(game_status.open_round, true);
            assert_eq!(game_status.unfinished_round, true);
            assert_eq!(game_status.slug, game.slug.unwrap());
        } else {
            assert!(false, "Message was not a string");
        }

        drop(stream);

        srv.stop().await;

        diesel::delete(rounds::table).execute(&conn).unwrap();
        diesel::delete(games::table).execute(&conn).unwrap();
    }

    #[actix_rt::test]
    async fn test_create_round_as_different_owner() {
        let pool = new_pool();
        let conn = get_conn(&pool).unwrap();

        let game: Game = diesel::insert_into(games::table)
            .values(NewGame {
                slug: Some("abc123".to_string()),
            })
            .get_result(&conn)
            .unwrap();
        let claim = PrivateClaim::new(game.id, game.slug.unwrap().clone(), game.id, Role::Owner);
        let token = create_jwt(claim).unwrap();

        diesel::update(games::dsl::games.find(game.id))
            .set(games::dsl::creator.eq(token.clone()))
            .execute(&conn)
            .unwrap();

        let wrong_claim =
            PrivateClaim::new(game.id + 1, "abc222".to_string(), game.id + 1, Role::Owner);
        let token = create_jwt(wrong_claim).unwrap();

        let (status, _): (u16, ErrorResponse) = test_post(
            "/api/rounds",
            CreateRoundRequest {
                player_one: "Boxer".to_string(),
                player_two: "Idra".to_string(),
            },
            Some(token),
        )
        .await;

        assert_eq!(status, 403);

        let round_results: Vec<Round> = rounds::dsl::rounds.load::<Round>(&conn).unwrap();
        assert_eq!(round_results.len(), 0);

        diesel::delete(games::table).execute(&conn).unwrap();
    }

    #[actix_rt::test]
    async fn test_create_round_as_invalid_owner_for_same_game() {
        let pool = new_pool();
        let conn = get_conn(&pool).unwrap();

        let game: Game = diesel::insert_into(games::table)
            .values(NewGame {
                slug: Some("abc123".to_string()),
            })
            .get_result(&conn)
            .unwrap();
        let claim = PrivateClaim::new(game.id, game.slug.unwrap().clone(), game.id, Role::Owner);
        let token = create_jwt(claim).unwrap();

        diesel::update(games::dsl::games.find(game.id))
            .set(games::dsl::creator.eq(token.clone()))
            .execute(&conn)
            .unwrap();

        let wrong_claim = PrivateClaim::new(game.id, "abc222".to_string(), game.id, Role::Owner);
        let token = create_jwt(wrong_claim).unwrap();

        let (status, _): (u16, ErrorResponse) = test_post(
            "/api/rounds",
            CreateRoundRequest {
                player_one: "Boxer".to_string(),
                player_two: "Idra".to_string(),
            },
            Some(token),
        )
        .await;

        assert_eq!(status, 403);

        let round_results: Vec<Round> = rounds::dsl::rounds.load::<Round>(&conn).unwrap();
        assert_eq!(round_results.len(), 0);

        diesel::delete(games::table).execute(&conn).unwrap();
    }

    #[actix_rt::test]
    async fn test_create_round_as_player() {
        let pool = new_pool();
        let conn = get_conn(&pool).unwrap();

        let game: Game = diesel::insert_into(games::table)
            .values(NewGame {
                slug: Some("abc123".to_string()),
            })
            .get_result(&conn)
            .unwrap();
        let claim = PrivateClaim::new(game.id, game.slug.unwrap().clone(), game.id, Role::Player);
        let token = create_jwt(claim).unwrap();

        diesel::update(games::dsl::games.find(game.id))
            .set(games::dsl::creator.eq(token.clone()))
            .execute(&conn)
            .unwrap();

        let (status, _): (u16, ErrorResponse) = test_post(
            "/api/rounds",
            CreateRoundRequest {
                player_one: "Boxer".to_string(),
                player_two: "Idra".to_string(),
            },
            Some(token),
        )
        .await;

        assert_eq!(status, 403);

        let round_results: Vec<Round> = rounds::dsl::rounds.load::<Round>(&conn).unwrap();
        assert_eq!(round_results.len(), 0);

        diesel::delete(games::table).execute(&conn).unwrap();
    }
}
