use actix_identity::Identity;
use actix_web::web::{block, Data, Json, Path};
use serde::{Deserialize, Serialize};

use crate::auth::identity_matches_game_id;
use crate::db::{get_conn, models::Game, PgPool};
use crate::errors;

#[derive(Deserialize, Serialize)]
pub struct StatusResponse {
    slug: String,
}

pub async fn status(
    id: Identity,
    game_id: Path<i32>,
    pool: Data<PgPool>,
) -> Result<Json<StatusResponse>, errors::Error> {
    let game_id = game_id.into_inner();
    identity_matches_game_id(id, game_id)?;

    let connection = get_conn(&pool)?;
    let game = block(move || Game::find_by_id(&connection, game_id)).await?;

    Ok(Json(StatusResponse {
        slug: game.slug.unwrap_or_else(|| String::new()),
    }))
}

#[cfg(test)]
mod tests {
    use diesel::{self, RunQueryDsl};

    use crate::auth::{PrivateClaim, Role};
    use crate::db::{get_conn, models::Game, new_pool};
    use crate::schema::games;
    use crate::tests::helpers::tests::{get_auth_token, test_get};

    use super::StatusResponse;

    #[derive(Insertable)]
    #[table_name = "games"]
    struct NewGame {
        slug: Option<String>,
    }

    #[actix_rt::test]
    async fn test_get_game_status() {
        let pool = new_pool();
        let conn = get_conn(&pool).unwrap();

        let game: Game = diesel::insert_into(games::table)
            .values(NewGame {
                slug: Some("abc123".to_string()),
            })
            .get_result(&conn)
            .unwrap();

        let token = get_auth_token(PrivateClaim::new(
            game.id,
            game.slug.unwrap(),
            game.id,
            Role::Owner,
        ));
        let res: (u16, StatusResponse) =
            test_get(&format!("/api/games/{}", game.id), Some(token)).await;
        assert_eq!(res.0, 200);

        assert_eq!(res.1.slug, "abc123");
    }
}
