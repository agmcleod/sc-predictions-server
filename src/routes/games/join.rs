use actix_web::{web, Error, HttpResponse, Result};
use serde::Deserialize;
use validator::Validate;

use crate::db::{
    get_conn,
    models::{Game, User},
    PgPool,
};

#[derive(Clone, Deserialize, Validate)]
pub struct JoinRequest {
    #[validate(length(min = "3"))]
    name: String,
    #[validate(length(equal = "6"))]
    slug: String,
}

pub async fn join(
    pool: web::Data<PgPool>,
    params: web::Json<JoinRequest>,
) -> Result<HttpResponse, Error> {
    match params.validate() {
        Ok(_) => {
            let connection = get_conn(&pool).unwrap();
            let game = Game::find_by_slug(&connection, params.slug)?;
            let user = User::create(&connection, params.name, game.id)?;

            Ok(HttpResponse::Ok().json(user))
        }
        Err(e) => Box::new(futures::future::ok(
            Error::ValidationError(e).error_response(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use std;

    use actix_web::http;
    use diesel;
    use serde_json;

    use crate::app_tests::{get_server, POOL};
    use crate::db::{get_conn, models::User};
    use crate::schema::games;

    use super::JoinRequest;

    #[derive(Insertable)]
    #[table_name = "games"]
    struct NewGame {
        slug: String,
        locked: bool,
    }

    #[test]
    fn test_join_game() {
        let conn = get_conn(&POOL).unwrap();

        let rows = conn
            .query(
                "INSERT INTO games (slug, locked) VALUES ('abc123', false) RETURNING *",
                &[],
            )
            .unwrap();

        let game = diesel::insert_into(games::table)
            .values(NewGame {
                slug: "abc123".to_string(),
                locked: false,
            })
            .get_result(&conn)?;

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
