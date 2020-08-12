use actix_identity::Identity;
use actix_web::{
    web::{block, Data, Json},
    Result,
};
use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::auth::{get_claim_from_identity, Role};
use crate::db::{
    get_conn,
    models::{Game, Round},
    PgPool,
};
use crate::errors::Error;
use crate::validate::validate;

#[derive(Clone, Deserialize, Serialize, Validate)]
pub struct CreateRoundRequest {
    #[validate(length(min = "1"))]
    player_one: String,
    #[validate(length(min = "1"))]
    player_two: String,
}

pub async fn create(
    id: Identity,
    pool: Data<PgPool>,
    params: Json<CreateRoundRequest>,
) -> Result<Json<Round>, Error> {
    validate(&params)?;

    let (claim, token) = get_claim_from_identity(id)?;
    if claim.role != Role::Owner {
        return Err(Error::Forbidden);
    }

    let conn = get_conn(&pool).unwrap();

    let game = Game::find_by_id(&conn, claim.game_id).map_err(|err| match err {
        // if the game didnt exist, return a forbidden error
        Error::NotFound(_) => Error::Forbidden,
        _ => err,
    })?;

    if game.creator.is_none() || game.creator.unwrap() != token {
        return Err(Error::Forbidden);
    }

    let round = block(move || {
        Round::create(
            &conn,
            claim.game_id,
            params.player_one.clone(),
            params.player_two.clone(),
        )
    })
    .await?;

    Ok(Json(round))
}

#[cfg(test)]
mod tests {
    use diesel::{self, ExpressionMethods, QueryDsl, RunQueryDsl};

    use crate::auth::{create_jwt, PrivateClaim, Role};
    use crate::db::{
        get_conn,
        models::{Game, Round},
        new_pool,
    };
    use crate::errors::ErrorResponse;
    use crate::schema::{games, rounds};
    use crate::tests::helpers::tests::test_post;

    use super::CreateRoundRequest;

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
        let claim = PrivateClaim::new(game.id, game.slug.unwrap().clone(), game.id, Role::Owner);
        let token = create_jwt(claim).unwrap();

        diesel::update(games::dsl::games.find(game.id))
            .set(games::dsl::creator.eq(token.clone()))
            .execute(&conn)
            .unwrap();

        let (status, round): (u16, Round) = test_post(
            "/api/rounds",
            CreateRoundRequest {
                player_one: "Boxer".to_string(),
                player_two: "Idra".to_string(),
            },
            Some(token),
        )
        .await;

        assert_eq!(status, 200);
        assert_eq!(round.player_one, "Boxer");
        assert_eq!(round.player_two, "Idra");

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
