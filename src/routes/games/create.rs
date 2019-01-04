use actix_web::{AsyncResponder, HttpMessage, HttpRequest, HttpResponse, FutureResponse, Result, Error};
use actix::prelude::{Handler, Message};
use futures::Future;

use db::{DbExecutor, models::Game, get_conn};
use app::AppState;

#[derive(Deserialize, Serialize)]
pub struct CreateGame {
    question_ids: Vec<i32>,
}

impl Message for CreateGame {
    type Result = Result<Game, Error>;
}

impl Handler<CreateGame> for DbExecutor {
    type Result = Result<Game, Error>;

    fn handle(&mut self, _: CreateGame, _: &mut Self::Context) -> Self::Result {
        let connection = get_conn(&self.0).unwrap();
        let game = Game::create(&connection).map_err(|err| {
            error!("Failed to create game - {}", err.to_string());
            println!("Failed to create game - {}", err.to_string());
        })
        .unwrap();

        Ok(game)
    }
}

pub fn create(req: &HttpRequest<AppState>) -> FutureResponse<HttpResponse> {
    req.json()
        .from_err()
        .and_then(|val: CreateGame| {
            req.state()
                .db
                .send(val)
                .from_err()
                .and_then(|res| match res {
                    Ok(game) => Ok(HttpResponse::Ok().json(game)),
                    Err(_) => Ok(HttpResponse::InternalServerError().into())
                })
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

    use super::{CreateGame};

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
        let req = srv.client(http::Method::POST, "/api/games")
            .json(CreateGame{ question_ids: question_ids.clone() })
            .unwrap();
        let res = srv.execute(req.send())
            .map_err(|err| {
                println!("{}", err);
            })
            .unwrap();
        assert!(res.status().is_success());

        let bytes = srv.execute(res.body()).unwrap();
        let body = std::str::from_utf8(&bytes).unwrap();
        let game: serde_json::Result<Game> = serde_json::from_str(body);

        assert!(game.is_ok());

        let conn = get_conn(&POOL).unwrap();
        let game = game.unwrap();
        conn.execute("DELETE FROM games WHERE id = $1", &[&game.id]).unwrap();
        conn.execute("DELETE FROM questions WHERE id IN $1", &[&question_ids]).unwrap();
    }
}
