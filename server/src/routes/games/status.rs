use actix_identity::Identity;
use actix_web::web::{block, Data, Json, Path};
use diesel::{BelongingToDsl, RunQueryDsl};
use serde::{Deserialize, Serialize};

use auth::identity_matches_game_id;
use db::{
    get_conn,
    models::{Game, Round},
    PgPool,
};
use errors;

#[derive(Deserialize, Serialize)]
pub struct StatusResponse {
    slug: String,
    open_round: bool,
    unfinished_round: bool,
}

pub async fn status(
    id: Identity,
    game_id: Path<i32>,
    pool: Data<PgPool>,
) -> Result<Json<StatusResponse>, errors::Error> {
    let game_id = game_id.into_inner();
    identity_matches_game_id(id, game_id)?;

    let connection = get_conn(&pool)?;
    let (game, rounds) = block(move || {
        let game = Game::find_by_id(&connection, game_id)?;
        let rounds = Round::belonging_to(&game).load::<Round>(&connection)?;

        Ok((game, rounds))
    })
    .await?;

    Ok(Json(StatusResponse {
        slug: game.slug.unwrap_or_else(|| "".to_string()),
        open_round: rounds.iter().fold(false, |result: bool, round: &Round| {
            // if there is a round that's not locked, we want to return true
            if !round.locked {
                return true;
            }
            result
        }),
        unfinished_round: rounds.iter().fold(false, |result: bool, round: &Round| {
            if !round.finished {
                return true;
            }
            result
        }),
    }))
}

#[cfg(test)]
mod tests {
    use diesel::{self, RunQueryDsl};

    use crate::tests::helpers::tests::{get_auth_token, test_get};
    use auth::{PrivateClaim, Role};
    use db::{
        get_conn,
        models::Game,
        new_pool,
        schema::{games, rounds},
    };

    use super::StatusResponse;

    #[derive(Insertable)]
    #[table_name = "games"]
    struct NewGame {
        slug: Option<String>,
    }

    #[derive(Insertable)]
    #[table_name = "rounds"]
    struct NewRound {
        pub player_one: String,
        pub player_two: String,
        pub game_id: i32,
        pub locked: bool,
        pub finished: bool,
    }

    #[actix_rt::test]
    async fn test_get_game_status() {
        let pool = new_pool();
        let conn = get_conn(&pool).unwrap();

        let game: Game = diesel::insert_into(games::table)
            .values(NewGame {
                slug: Some("abc123".to_string()),
            })
            .get_result(&conn)
            .unwrap();

        diesel::insert_into(rounds::table)
            .values(NewRound {
                player_one: "one".to_string(),
                player_two: "two".to_string(),
                game_id: game.id,
                locked: true,
                finished: false,
            })
            .execute(&conn)
            .unwrap();

        diesel::insert_into(rounds::table)
            .values(NewRound {
                player_one: "one".to_string(),
                player_two: "two".to_string(),
                game_id: game.id,
                locked: true,
                finished: false,
            })
            .execute(&conn)
            .unwrap();

        // isnt locked, but wrong game id
        let other_game: Game = diesel::insert_into(games::table)
            .values(NewGame {
                slug: Some("dfg888".to_string()),
            })
            .get_result(&conn)
            .unwrap();
        diesel::insert_into(rounds::table)
            .values(NewRound {
                player_one: "one".to_string(),
                player_two: "two".to_string(),
                game_id: other_game.id,
                locked: false,
                finished: false,
            })
            .execute(&conn)
            .unwrap();

        let token = get_auth_token(PrivateClaim::new(
            game.id,
            game.slug.unwrap(),
            game.id,
            Role::Owner,
        ));
        let res: (u16, StatusResponse) =
            test_get(&format!("/api/games/{}", game.id), Some(token)).await;
        assert_eq!(res.0, 200);

        assert_eq!(res.1.slug, "abc123");
        assert_eq!(res.1.open_round, false);
        assert_eq!(res.1.unfinished_round, true);

        diesel::delete(rounds::table).execute(&conn).unwrap();
        diesel::delete(games::table).execute(&conn).unwrap();
    }

    #[actix_rt::test]
    async fn test_get_game_status_open_round() {
        let pool = new_pool();
        let conn = get_conn(&pool).unwrap();

        let game: Game = diesel::insert_into(games::table)
            .values(NewGame {
                slug: Some("abc123".to_string()),
            })
            .get_result(&conn)
            .unwrap();

        diesel::insert_into(rounds::table)
            .values(NewRound {
                player_one: "one".to_string(),
                player_two: "two".to_string(),
                game_id: game.id,
                locked: true,
                finished: true,
            })
            .execute(&conn)
            .unwrap();

        diesel::insert_into(rounds::table)
            .values(NewRound {
                player_one: "one".to_string(),
                player_two: "two".to_string(),
                game_id: game.id,
                locked: true,
                finished: true,
            })
            .execute(&conn)
            .unwrap();

        diesel::insert_into(rounds::table)
            .values(NewRound {
                player_one: "one".to_string(),
                player_two: "two".to_string(),
                game_id: game.id,
                locked: false,
                finished: true,
            })
            .execute(&conn)
            .unwrap();

        let token = get_auth_token(PrivateClaim::new(
            game.id,
            game.slug.unwrap(),
            game.id,
            Role::Owner,
        ));
        let res: (u16, StatusResponse) =
            test_get(&format!("/api/games/{}", game.id), Some(token)).await;
        assert_eq!(res.0, 200);

        assert_eq!(res.1.slug, "abc123");
        assert_eq!(res.1.open_round, true);
        assert_eq!(res.1.unfinished_round, false);

        diesel::delete(rounds::table).execute(&conn).unwrap();
        diesel::delete(games::table).execute(&conn).unwrap();
    }
}
