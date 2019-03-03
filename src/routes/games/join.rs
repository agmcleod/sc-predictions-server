use actix::prelude::{Handler, Message};
use actix_web::{
    error, AsyncResponder, Error, FutureResponse, HttpRequest, HttpResponse, Json, Result,
};
use futures::Future;

use app::AppState;
use db::{
    get_conn,
    models::{Game, User},
    DbExecutor,
};

#[derive(Clone, Deserialize, Serialize)]
pub struct JoinRequest {
    name: String,
    game_slug: String,
}

impl Message for JoinRequest {
    type Result = Result<User, Error>;
}

impl Handler<JoinRequest> for DbExecutor {
    type Result = Result<User, Error>;

    fn handle(&mut self, request: JoinRequest, _: &mut Self::Context) -> Self::Result {
        use postgres_mapper::FromPostgresRow;
        let connection = get_conn(&self.0).unwrap();
        connection
            .query("SELECT * FROM games WHERE slug = $1", &[&request.game_slug])
            .map_err(|err| error::ErrorInternalServerError(err))
            .and_then(|rows| {
                Game::from_postgres_row(rows.get(0))
                    .map_err(|err| error::ErrorInternalServerError(err))
            })
            .and_then(|game| User::create(&connection, request.name, game.id))
            .map_err(|err| error::ErrorInternalServerError(err))
    }
}

pub fn join(
    (req, params): (HttpRequest<AppState>, Json<JoinRequest>),
) -> FutureResponse<HttpResponse> {
    req.state()
        .db
        .send(params.0.clone())
        .from_err()
        .and_then(|res| match res {
            Ok(user) => Ok(HttpResponse::Ok().json(user)),
            Err(err) => Ok(HttpResponse::InternalServerError()
                .body(err.to_string())
                .into()),
        })
        .responder()
}

#[cfg(test)]
mod tests {
    use std;

    use actix_web::{http, HttpMessage};
    use postgres_mapper::FromPostgresRow;
    use serde_json;

    use app_tests::{get_server, POOL};
    use db::{
        get_conn,
        models::{Game, User},
    };

    use super::JoinRequest;

    #[test]
    fn test_join_game() {
        let conn = get_conn(&POOL).unwrap();

        let rows = conn
            .query(
                "INSERT INTO games (slug, locked) VALUES ('abc123', false) RETURNING *",
                &[],
            )
            .unwrap();

        let game = Game::from_postgres_row(rows.get(0)).unwrap();

        let mut srv = get_server();
        let req = srv
            .client(http::Method::POST, "/api/games/join")
            .json(JoinRequest {
                name: "agmcleod".to_string(),
                game_slug: game.slug,
            })
            .unwrap();

        let res = srv.execute(req.send()).unwrap();
        assert!(res.status().is_success());

        let bytes = srv.execute(res.body()).unwrap();
        let body = std::str::from_utf8(&bytes).unwrap();
        let user: serde_json::Result<User> = serde_json::from_str(body);

        assert!(user.is_ok());

        let rows = conn.query("SELECT * FROM users", &[]).unwrap();
        assert_eq!(rows.len(), 1);

        conn.execute("DELETE FROM users", &[]).unwrap();
        conn.execute("DELETE FROM games", &[]).unwrap();
    }
}
