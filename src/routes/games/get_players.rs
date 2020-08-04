use actix_identity::Identity;
use actix_web::{
    web::{block, Data, Json, Path},
    HttpResponse,
};

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
    use actix_web::{
        web::{Data, Json, Path},
        HttpResponse,
    };
    use diesel::{self, ExpressionMethods, QueryDsl, RunQueryDsl};

    use crate::auth::{create_jwt, PrivateClaim};
    use crate::db::{
        get_conn,
        models::{Game, User},
        new_pool,
    };
    use crate::errors::Error;
    use crate::schema::{games, users};
    use crate::tests::helpers::tests::get_identity;

    use super::get_players;

    #[derive(Insertable)]
    #[table_name = "users"]
    struct NewUser {
        user_name: String,
        game_id: i32,
    }

    async fn get_players_fn(game_id: i32) -> Result<Json<Vec<User>>, Error> {
        let pool = new_pool();
        let identity = get_identity().await;
        identity.remember(
            create_jwt(PrivateClaim::new(game_id, "agmcleod".to_string(), game_id)).unwrap(),
        );
        get_players(identity, Path::from(game_id), Data::new(pool)).await
    }

    #[actix_rt::test]
    async fn test_get_players() {
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
                game_id: game_2.id,
            })
            .execute(&conn)
            .unwrap();

        let players = get_players_fn(game.id).await;

        assert_eq!(players.unwrap().len(), 1);

        diesel::delete(users::table).execute(&conn).unwrap();
        diesel::delete(games::table).execute(&conn).unwrap();
    }
}
