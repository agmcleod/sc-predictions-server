use actix::prelude::{Handler, Message};
use actix_web::{
    AsyncResponder, FutureResponse, HttpRequest, HttpResponse, Json, ResponseError, Result,
};
use futures::Future;
use validator::Validate;

use app::AppState;
use db::{
    get_conn,
    models::{Game, User},
    DbExecutor,
};
use errors::{DBError, Error};

#[derive(Clone, Deserialize, Serialize, Validate)]
pub struct JoinRequest {
    #[validate(length(min = "3"))]
    name: String,
    #[validate(length(equal = "6"))]
    slug: String,
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
            .query("SELECT * FROM games WHERE slug = $1", &[&request.slug])
            .map_err(|err| Error::DBError(DBError::PGError(err)))
            .and_then(|rows| {
                if rows.len() == 0 {
                    Err(Error::DBError(DBError::NoRecord))
                } else {
                    Game::from_postgres_row(rows.get(0))
                        .map_err(|err| Error::DBError(DBError::MapError(err.to_string())))
                        .and_then(|game| User::create(&connection, request.name, game.id))
                }
            })
    }
}

pub fn join(
    (req, params): (HttpRequest<AppState>, Json<JoinRequest>),
) -> FutureResponse<HttpResponse> {
    match params.validate() {
        Ok(_) => req
            .state()
            .db
            .send(params.0.clone())
            .from_err()
            .and_then(|res| match res {
                Ok(user) => Ok(HttpResponse::Ok().json(user)),
                Err(err) => Ok(err.error_response()),
            })
            .responder(),
        Err(e) => Box::new(futures::future::ok(
            Error::ValidationError(e).error_response(),
        )),
    }
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
                slug: game.slug,
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

    #[test]
    fn test_game_not_found() {
        let mut srv = get_server();
        let req = srv
            .client(http::Method::POST, "/api/games/join")
            .json(JoinRequest {
                name: "agmcleod".to_string(),
                slug: "null".to_string(),
            })
            .unwrap();

        let res = srv.execute(req.send()).unwrap();
        assert_eq!(res.status(), http::StatusCode::UNPROCESSABLE_ENTITY);
    }
}
