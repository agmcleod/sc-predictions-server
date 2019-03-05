use actix::prelude::{Handler, Message};
use actix_web::{
    error, AsyncResponder, Error, FutureResponse, HttpRequest, HttpResponse, Json, Result,
};
use futures::Future;

use app::AppState;
use db::{
    get_conn,
    models::{Game, GameQuestion},
    DbExecutor,
};

#[derive(Clone, Deserialize, Serialize)]
pub struct CreateGameRequest {
    question_ids: Vec<i32>,
}

impl Message for CreateGameRequest {
    type Result = Result<Game, Error>;
}

impl Handler<CreateGameRequest> for DbExecutor {
    type Result = Result<Game, Error>;

    fn handle(&mut self, request: CreateGameRequest, _: &mut Self::Context) -> Self::Result {
        let connection = get_conn(&self.0).unwrap();
        let result = connection
            .transaction()
            .map_err(|err| error::ErrorInternalServerError(err))
            .and_then(|transaction| {
                Ok(Game::create(&transaction).and_then(|game| {
                    for question_id in &request.question_ids {
                        GameQuestion::create(&transaction, game.id, *question_id)?;
                    }
                    transaction
                        .commit()
                        .and_then(|()| Ok(game))
                        .map_err(|err| error::ErrorInternalServerError(err))
                })?)
            });

        result
    }
}

pub fn create(
    (req, params): (HttpRequest<AppState>, Json<CreateGameRequest>),
) -> FutureResponse<HttpResponse> {
    req.state()
        .db
        .send(params.0.clone())
        .from_err()
        .and_then(|res| match res {
            Ok(game) => Ok(HttpResponse::Ok().json(game)),
            Err(err) => Ok(HttpResponse::from(err)),
        })
        .responder()
}

#[cfg(test)]
mod tests {
    use std;

    use actix_web::{http, HttpMessage};
    use chrono::{TimeZone, Utc};
    use serde_json;

    use app_tests::{get_server, POOL};
    use db::{get_conn, models::Game};

    use super::CreateGameRequest;

    #[test]
    fn test_create_game() {
        let conn = get_conn(&POOL).unwrap();

        let rows = conn.query(
            "INSERT INTO questions (body, created_at, updated_at) VALUES ('This is the question', $1, $2) RETURNING *",
            &[
                &Utc.ymd(2017, 12, 10).and_hms(0, 0, 0),
                &Utc.ymd(2017, 12, 10).and_hms(0, 0, 0),
            ],
        ).unwrap();

        let question_ids: Vec<i32> = rows.iter().map(|row| row.get(0)).collect();

        let mut srv = get_server();
        let req = srv
            .client(http::Method::POST, "/api/games")
            .json(CreateGameRequest {
                question_ids: question_ids.clone(),
            })
            .map_err(|err| {
                println!("Req error {:?}", err);
            })
            .unwrap();
        let res = srv
            .execute(req.send())
            .map_err(|err| {
                println!("Res error {:?}", err);
            })
            .unwrap();

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
