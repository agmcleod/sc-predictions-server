use actix_web::{AsyncResponder, HttpRequest, HttpResponse, FutureResponse, Result, Error};
use actix::prelude::{Handler, Message};
use futures::Future;
use serde_json;

use db::{DbExecutor, models::Game, get_conn};
use app::AppState;

#[derive(Deserialize)]
pub struct GameQuestionIdPair {
    pub game_id: i32,
    pub question_id: i32,
}

pub struct CreateGame {
    join_ids: Vec<GameQuestionIdPair>,
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
    let game_questions: Vec<GameQuestionIdPair> = serde_json::from_str(&req.match_info()["game_questions"]).unwrap();
    req.state()
        .db
        .send(CreateGame{ join_ids: game_questions })
        .from_err()
        .and_then(|res| match res {
            Ok(game) => Ok(HttpResponse::Ok().json(game)),
            Err(_) => Ok(HttpResponse::InternalServerError().into())
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

    #[test]
    fn test_create_game() {
        let mut srv = get_server();
        let req = srv.client(http::Method::POST, "/api/games")
            .finish()
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
    }
}
