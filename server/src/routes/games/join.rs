use actix::Addr;
use actix_web::{
    web::{block, Data, Json},
    Result,
};
use serde::{Deserialize, Serialize};
use serde_json::to_value;
use validator::Validate;

use db::{
    get_conn,
    models::{Game, User},
    PgPool,
};
use errors::Error;

use crate::validate::validate;
use crate::websocket::{MessageToClient, Server};

#[derive(Clone, Deserialize, Serialize, Validate)]
pub struct JoinRequest {
    #[validate(length(min = "3"))]
    name: String,
    #[validate(length(equal = "6"))]
    slug: String,
}

pub async fn join(
    pool: Data<PgPool>,
    websocket_srv: Data<Addr<Server>>,
    params: Json<JoinRequest>,
) -> Result<Json<User>, Error> {
    validate(&params)?;
    let connection = get_conn(&pool).unwrap();

    let (new_user, users, game_id) = block(move || {
        let game = Game::find_by_slug(&connection, &params.slug)?;
        if User::find_by_game_id_and_name(&connection, game.id, &params.name).is_ok() {
            return Err(Error::UnprocessableEntity("Username is taken".to_string()));
        }
        let new_user = User::create(&connection, params.name.clone(), game.id)?;
        let users = User::find_all_by_game_id(&connection, game.id)?;
        Ok((new_user, users, game.id))
    })
    .await?;

    if let Ok(value) = to_value(users) {
        let msg = MessageToClient::new("/players", game_id, value);
        websocket_srv.do_send(msg);
    }

    Ok(Json(new_user))
}

#[cfg(test)]
mod tests {
    use actix_web::client::Client;
    use diesel::RunQueryDsl;

    use db::{
        get_conn,
        models::{Game, User},
        new_pool,
        schema::{games, users},
    };
    use errors::ErrorResponse;
    use futures::StreamExt;

    use super::JoinRequest;
    use crate::tests::helpers::tests::{get_test_server, test_post};

    #[derive(Insertable)]
    #[table_name = "games"]
    struct NewGame {
        slug: String,
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
            })
            .get_result(&conn)
            .unwrap();

        let srv = get_test_server();

        let client = Client::default();
        let ws = client.ws(srv.url("/ws/"));

        let ws_res = ws.connect().await.unwrap();

        // let (sink, stream) = ws_res.1.split();

        let req = srv.post("/api/games/join");
        let mut res = req
            .send_json(&JoinRequest {
                name: "agmcleod".to_string(),
                slug: game.slug.unwrap(),
            })
            .await
            .unwrap();

        assert_eq!(res.status().as_u16(), 200);

        let user: User = res.json::<User>().await.unwrap();

        assert_eq!(user.user_name, "agmcleod");

        srv.stop().await;

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
    async fn test_join_game_with_duplicate_name() {
        let pool = new_pool();
        let conn = get_conn(&pool).unwrap();

        let game: Game = diesel::insert_into(games::table)
            .values(NewGame {
                slug: "newgam".to_string(),
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
