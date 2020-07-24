use actix_web::{error, web, HttpResponse, Result};
use diesel::result::Error;
use serde::{Deserialize, Serialize};

use crate::db::{
    get_conn,
    models::{Game, GameQuestion},
    PgPool,
};

#[derive(Clone, Deserialize, Serialize)]
pub struct CreateGameRequest {
    question_ids: Vec<i32>,
}

pub async fn create(
    pool: web::Data<PgPool>,
    params: web::Json<CreateGameRequest>,
) -> Result<HttpResponse, Error> {
    use diesel::connection::Connection;

    let connection = get_conn(&pool).unwrap();

    connection
        .transaction::<Game, Error, _>(|| {
            let game = Game::create(&connection)?;

            for question_id in &params.question_ids {
                GameQuestion::create(&connection, game.id, *question_id)?;
            }

            Ok(game)
        })
        .and_then(|game| Ok(HttpResponse::Ok().json(game)))
        .map_err(|err| error::ErrorInternalServerError(err))
}

#[cfg(test)]
mod tests {
    use std;

    use actix_web::http;
    use chrono::{TimeZone, Utc};
    use serde_json;

    use crate::db::{get_conn, models::Game};
    use crate::tests::helpers::tests::{test_post, POOL};

    use super::CreateGameRequest;

    #[actix_rt::test]
    async fn test_create_game() {
        let conn = get_conn(&POOL).unwrap();
        let rows = conn.query(
            "INSERT INTO questions (body, created_at, updated_at) VALUES ('This is the question', $1, $2) RETURNING *",
            &[
                &Utc.ymd(2017, 12, 10).and_hms(0, 0, 0),
                &Utc.ymd(2017, 12, 10).and_hms(0, 0, 0),
            ],
        ).unwrap();

        let question_ids: Vec<i32> = rows.iter().map(|row| row.get(0)).collect();

        let res = test_post(
            "/api/games",
            CreateGameRequest {
                question_ids: question_ids.clone(),
            },
        )
        .await;

        assert!(res.status().is_success());

        let bytes = srv.execute(res.body()).unwrap();
        let body = std::str::from_utf8(&bytes).unwrap();
        let game: serde_json::Result<Game> = serde_json::from_str(body);

        assert!(game.is_ok());

        let game_questions = conn.query("SELECT * FROM game_questions", &[]).unwrap();
        assert_eq!(game_questions.len(), 1);

        let game = game.unwrap();
        conn.execute("DELETE FROM game_questions", &[]).unwrap();

        conn.execute("DELETE FROM games WHERE id = $1", &[&game.id])
            .unwrap();

        conn.execute("DELETE FROM questions WHERE id = ANY($1)", &[&question_ids])
            .unwrap();
    }
}
