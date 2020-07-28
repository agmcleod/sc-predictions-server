use actix_web::{web, HttpResponse, Result};
use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::db::{
    get_conn,
    models::{Game, User},
    PgPool,
};
use crate::errors::Error;
use crate::validate::validate;

#[derive(Clone, Deserialize, Serialize, Validate)]
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
    validate(&params)?;
    let connection = get_conn(&pool).unwrap();
    let game = Game::find_by_slug(&connection, &params.slug)?;
    let user = User::create(&connection, params.name.clone(), game.id)?;

    Ok(HttpResponse::Ok().json(user))
}

#[cfg(test)]
mod tests {
    use diesel;

    use crate::db::{
        get_conn,
        models::{Game, User},
        new_pool,
    };
    use crate::schema::{games, users};
    use crate::tests::helpers::tests::test_post;

    use super::JoinRequest;
    use diesel::RunQueryDsl;

    #[derive(Insertable)]
    #[table_name = "games"]
    struct NewGame {
        slug: String,
        locked: bool,
    }

    #[actix_rt::test]
    async fn test_join_game() {
        let pool = new_pool();
        let conn = get_conn(&pool).unwrap();

        let game: Game = diesel::insert_into(games::table)
            .values(NewGame {
                slug: "abc123".to_string(),
                locked: false,
            })
            .get_result(&conn)
            .unwrap();

        let res = test_post(
            "/api/games/join",
            JoinRequest {
                name: "agmcleod".to_string(),
                slug: game.slug.unwrap(),
            },
        )
        .await;

        assert_eq!(res.0, 200);

        let user: User = res.1;

        assert_eq!(user.user_name, "agmcleod");

        diesel::delete(games::table).execute(&conn).unwrap();
        diesel::delete(users::table).execute(&conn).unwrap();
    }

    #[actix_rt::test]
    async fn test_game_not_found() {
        let res: (u16, User) = test_post(
            "/api/games/join",
            JoinRequest {
                name: "agmcleod".to_string(),
                slug: "null".to_string(),
            },
        )
        .await;

        assert_eq!(res.0, 422);
    }
}
