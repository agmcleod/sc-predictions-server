use actix_web::{web, HttpResponse};

use crate::db::{get_conn, models::User, PgPool};
use crate::errors;

pub async fn get_players(pool: web::Data<PgPool>) -> Result<HttpResponse, errors::Error> {
    let game_id = 1;
    let connection = get_conn(&pool)?;
    let users = web::block(move || User::find_all_by_game_id(&connection, game_id)).await?;

    Ok(HttpResponse::Ok().json(users))
}

#[cfg(test)]
mod tests {
    #[actix_rt::test]
    async fn test_get_players() {}
}
