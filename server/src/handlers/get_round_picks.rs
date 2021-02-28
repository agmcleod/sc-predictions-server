use actix_web::web::block;
use diesel::pg::PgConnection;
use diesel::r2d2::{ConnectionManager, PooledConnection};
use serde::{Deserialize, Serialize};

use db::models::{Round, UserAnswer, UserQuestion};
use errors::Error;

#[derive(Deserialize, PartialEq, Serialize)]
pub struct GetRoundPicksResponse {
    pub data: Vec<UserAnswer>,
    pub locked: bool,
}

pub async fn get_round_picks(
    connection: PooledConnection<ConnectionManager<PgConnection>>,
    game_id: i32,
) -> Result<GetRoundPicksResponse, Error> {
    let (user_questions, locked) = block(move || {
        let round = Round::get_active_round_by_game_id(&connection, game_id)?;

        let user_questions = UserQuestion::find_by_round(&connection, round.id)?;
        Ok((user_questions, round.locked))
    })
    .await?;

    Ok(GetRoundPicksResponse {
        data: user_questions,
        locked,
    })
}
