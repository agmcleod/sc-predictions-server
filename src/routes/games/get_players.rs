use actix_identity::Identity;
use actix_web::web::{block, Data, Json, Path};

use crate::auth::identity_matches_game_id;
use crate::db::{get_conn, models::User, PgPool};
use crate::errors;

pub async fn get_players(
    id: Identity,
    game_id: Path<i32>,
    pool: Data<PgPool>,
) -> Result<Json<Vec<User>>, errors::Error> {
    let game_id = game_id.into_inner();
    identity_matches_game_id(id, game_id)?;

    let connection = get_conn(&pool)?;
    let users = block(move || User::find_all_by_game_id(&connection, game_id)).await?;

    Ok(Json(users))
}

#[cfg(test)]
mod tests {
    use diesel::{self, RunQueryDsl};

    use crate::auth::{PrivateClaim, Role};
    use crate::db::{
        get_conn,
        models::{Game, User},
        new_pool,
    };
    use crate::errors::ErrorResponse;
    use crate::schema::{games, users};
    use crate::tests::helpers::tests::{get_auth_token, test_get};

    #[derive(Insertable)]
    #[table_name = "users"]
    struct NewUser {
        user_name: String,
        game_id: i32,
    }

    #[derive(Insertable)]
    #[table_name = "games"]
    struct NewGame {
        slug: Option<String>,
    }

    #[actix_rt::test]
    async fn test_get_players_as_player() {
        let pool = new_pool();
        let conn = get_conn(&pool).unwrap();

        let game: Game = diesel::insert_into(games::table)
            .default_values()
            .get_result(&conn)
            .unwrap();

        let game_2: Game = diesel::insert_into(games::table)
            .default_values()
            .get_result(&conn)
            .unwrap();

        let user: User = diesel::insert_into(users::table)
            .values(NewUser {
                user_name: "agmcleod".to_string(),
                game_id: game.id,
            })
            .get_result(&conn)
            .unwrap();

        diesel::insert_into(users::table)
            .values(NewUser {
                user_name: "agmcleod2".to_string(),
                game_id: game_2.id,
            })
            .execute(&conn)
            .unwrap();

        let cookie = get_auth_token(PrivateClaim::new(
            user.id,
            user.user_name,
            game.id,
            Role::Owner,
        ));
        let res = test_get(&format!("/api/games/{}/players", game.id), Some(cookie)).await;
        assert_eq!(res.0, 200);

        let body: Vec<User> = res.1;
        // only returns one, as second player is part of another game
        assert_eq!(body.len(), 1);

        diesel::delete(users::table).execute(&conn).unwrap();
        diesel::delete(games::table).execute(&conn).unwrap();
    }

    #[actix_rt::test]
    async fn test_get_players_as_owner() {
        let pool = new_pool();
        let conn = get_conn(&pool).unwrap();

        let game: Game = diesel::insert_into(games::table)
            .values(NewGame {
                slug: Some("abc123".to_string()),
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

        diesel::insert_into(users::table)
            .values(NewUser {
                user_name: "agmcleod2".to_string(),
                game_id: game.id,
            })
            .execute(&conn)
            .unwrap();

        let token = get_auth_token(PrivateClaim::new(
            game.id,
            game.slug.unwrap(),
            game.id,
            Role::Player,
        ));
        let res = test_get(&format!("/api/games/{}/players", game.id), Some(token)).await;
        assert_eq!(res.0, 200);

        let body: Vec<User> = res.1;
        // returns both as they are both apart of this game
        assert_eq!(body.len(), 2);

        diesel::delete(users::table).execute(&conn).unwrap();
        diesel::delete(games::table).execute(&conn).unwrap();
    }

    #[actix_rt::test]
    async fn test_get_players_forbidden() {
        let pool = new_pool();
        let conn = get_conn(&pool).unwrap();

        let game: Game = diesel::insert_into(games::table)
            .values(NewGame {
                slug: Some("abc123".to_string()),
            })
            .get_result(&conn)
            .unwrap();

        let cookie = get_auth_token(PrivateClaim::new(
            1,
            "".to_string(),
            game.id + 1,
            Role::Player,
        ));
        let res = test_get(&format!("/api/games/{}/players", game.id), Some(cookie)).await;
        assert_eq!(res.0, 403);

        let body: ErrorResponse = res.1;
        assert_eq!(body.errors.get(0).unwrap(), "Forbidden");

        diesel::delete(users::table).execute(&conn).unwrap();
        diesel::delete(games::table).execute(&conn).unwrap();
    }

    #[actix_rt::test]
    async fn test_get_players_unauthorized() {
        let pool = new_pool();
        let conn = get_conn(&pool).unwrap();

        let game: Game = diesel::insert_into(games::table)
            .values(NewGame {
                slug: Some("abc123".to_string()),
            })
            .get_result(&conn)
            .unwrap();

        let res = test_get(&format!("/api/games/{}/players", game.id), None).await;
        assert_eq!(res.0, 401);

        let body: ErrorResponse = res.1;
        assert_eq!(body.errors.get(0).unwrap(), "Unauthorized");

        diesel::delete(users::table).execute(&conn).unwrap();
        diesel::delete(games::table).execute(&conn).unwrap();
    }
}
