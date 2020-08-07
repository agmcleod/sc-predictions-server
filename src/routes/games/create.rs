use actix_identity::Identity;
use actix_web::{
    web::{block, Data, Json},
    Result,
};
use serde::{Deserialize, Serialize};

use crate::db::{
    get_conn,
    models::{Game, GameQuestion},
    PgPool,
};
use crate::errors::Error;

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
    id: Identity,
    pool: Data<PgPool>,
    params: Json<CreateGameRequest>,
) -> Result<Json<Game>, Error> {
    let game = block(move || create_db_records(pool, params)).await?;

    if let Some(token) = &game.creator {
        id.remember(token.clone());
    }

    Ok(Json(game))
}

#[cfg(test)]
mod tests {
    use diesel::{self, ExpressionMethods, QueryDsl, RunQueryDsl};

    use crate::db::{
        get_conn,
        models::{Game, Question},
        new_pool,
    };
    use crate::schema::{game_questions, games, questions};
    use crate::tests::helpers::tests::test_post;

    use super::CreateGameRequest;

    #[actix_rt::test]
    async fn test_create_game() {
        let pool = new_pool();
        let conn = get_conn(&pool).unwrap();
        let question = diesel::insert_into(questions::table)
            .values(questions::dsl::body.eq("This is the question"))
            .get_result::<Question>(&conn)
            .unwrap();

        let res = test_post(
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
