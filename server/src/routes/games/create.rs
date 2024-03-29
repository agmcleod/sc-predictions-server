use actix_web::{
    web::{block, Data, Json},
    Result,
};
use serde::{Deserialize, Serialize};

use db::{
    get_conn,
    models::{Game, GameQuestion},
    PgPool,
};
use errors::Error;

#[derive(Clone, Deserialize, Serialize)]
pub struct CreateGameRequest {
    question_ids: Vec<i32>,
}

fn create_db_records(pool: Data<PgPool>, params: Json<CreateGameRequest>) -> Result<Game, Error> {
    use diesel::connection::Connection;
    let connection = get_conn(&pool).unwrap();

    connection.transaction::<Game, Error, _>(|| {
        let game = Game::create(&connection)?;

        for question_id in &params.question_ids {
            GameQuestion::create(&connection, game.id, *question_id)?;
        }

        Ok(game)
    })
}

pub async fn create(
    pool: Data<PgPool>,
    params: Json<CreateGameRequest>,
) -> Result<Json<Game>, Error> {
    let res: Result<Game, Error> = block(move || create_db_records(pool, params)).await?;
    let game = res?;

    Ok(Json(game))
}

#[cfg(test)]
mod tests {
    use diesel::{self, ExpressionMethods, QueryDsl, RunQueryDsl};

    use crate::tests::helpers::tests::test_post;
    use db::{
        get_conn,
        models::{Game, Question},
        new_pool,
        schema::{game_questions, games, questions},
    };

    use super::CreateGameRequest;

    #[actix_rt::test]
    async fn test_create_game() {
        let pool = new_pool();
        let conn = get_conn(&pool).unwrap();
        let question = diesel::insert_into(questions::table)
            .values(questions::dsl::body.eq("This is the question"))
            .get_result::<Question>(&conn)
            .unwrap();

        let res: (u16, Game) = test_post(
            "/api/games",
            CreateGameRequest {
                question_ids: vec![question.id],
            },
            None,
        )
        .await;

        assert_eq!(res.0, 200);

        let gqs = game_questions::dsl::game_questions
            .select(game_questions::dsl::id)
            .load::<i32>(&conn)
            .unwrap();

        assert_eq!(gqs.len(), 1);

        diesel::delete(game_questions::table)
            .execute(&conn)
            .unwrap();
        diesel::delete(games::table).execute(&conn).unwrap();
        diesel::delete(questions::table).execute(&conn).unwrap();
    }
}
