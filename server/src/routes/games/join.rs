use actix_web::{
    web::{block, Data, Json},
    Result,
};
use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::validate::validate;
use db::{
    get_conn,
    models::{Game, User},
    PgPool,
};
use errors::Error;

#[derive(Clone, Deserialize, Serialize, Validate)]
pub struct JoinRequest {
    #[validate(length(min = "3"))]
    name: String,
    #[validate(length(equal = "6"))]
    slug: String,
}

pub async fn join(pool: Data<PgPool>, params: Json<JoinRequest>) -> Result<Json<User>, Error> {
    validate(&params)?;
    let connection = get_conn(&pool).unwrap();

    let user = block(move || {
        let game = Game::find_by_slug(&connection, &params.slug)?;
        if game.locked {
            return Err(Error::NotFound("Game is locked".to_string()));
        }
        if User::find_by_game_id_and_name(&connection, game.id, &params.name).is_ok() {
            return Err(Error::UnprocessableEntity("Username is taken".to_string()));
        }
        User::create(&connection, params.name.clone(), game.id)
    })
    .await?;

    Ok(Json(user))
}

#[cfg(test)]
mod tests {
    use crate::tests::helpers::tests::test_post;
    use db::{
        get_conn,
        models::{Game, User},
        new_pool,
        schema::{games, users},
    };
    use errors::ErrorResponse;

    use super::JoinRequest;
    use diesel::RunQueryDsl;

    #[derive(Insertable)]
    #[table_name = "games"]
    struct NewGame {
        slug: String,
        locked: bool,
    }

    #[derive(Insertable)]
    #[table_name = "users"]
    struct NewUser {
        user_name: String,
        game_id: i32,
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
            None,
        )
        .await;

        assert_eq!(res.0, 200);

        let user: User = res.1;

        assert_eq!(user.user_name, "agmcleod");

        diesel::delete(users::table).execute(&conn).unwrap();
        diesel::delete(games::table).execute(&conn).unwrap();
    }

    #[actix_rt::test]
    async fn test_game_not_found() {
        let res: (u16, ErrorResponse) = test_post(
            "/api/games/join",
            JoinRequest {
                name: "agmcleod".to_string(),
                slug: "-fake-".to_string(),
            },
            None,
        )
        .await;

        assert_eq!(res.0, 404);
    }

    #[actix_rt::test]
    async fn test_join_locked_game() {
        let pool = new_pool();
        let conn = get_conn(&pool).unwrap();

        diesel::insert_into(games::table)
            .values(NewGame {
                slug: "sluggd".to_string(),
                locked: true,
            })
            .execute(&conn)
            .unwrap();

        let res: (u16, ErrorResponse) = test_post(
            "/api/games/join",
            JoinRequest {
                slug: "sluggd".to_string(),
                name: "agmcleod".to_string(),
            },
            None,
        )
        .await;

        assert_eq!(res.0, 404);
        assert_eq!(res.1.errors[0], "Game is locked");

        diesel::delete(games::table).execute(&conn).unwrap();
    }

    #[actix_rt::test]
    async fn test_join_game_with_duplicate_name() {
        let pool = new_pool();
        let conn = get_conn(&pool).unwrap();

        let game: Game = diesel::insert_into(games::table)
            .values(NewGame {
                slug: "newgam".to_string(),
                locked: false,
            })
            .get_result(&conn)
            .unwrap();

        diesel::insert_into(users::table)
            .values(NewUser {
                user_name: "agmcleod".to_string(),
                game_id: game.id,
            })
            .execute(&conn)
            .unwrap();

        let res: (u16, ErrorResponse) = test_post(
            "/api/games/join",
            JoinRequest {
                slug: "newgam".to_string(),
                name: "agmcleod".to_string(),
            },
            None,
        )
        .await;

        assert_eq!(res.0, 422);
        assert_eq!(res.1.errors[0], "Username is taken");

        diesel::delete(users::table).execute(&conn).unwrap();
        diesel::delete(games::table).execute(&conn).unwrap();
    }
}