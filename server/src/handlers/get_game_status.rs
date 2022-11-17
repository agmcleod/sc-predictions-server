use actix_web::web::block;
use diesel::pg::PgConnection;
use diesel::r2d2::{ConnectionManager, PooledConnection};
use diesel::{BelongingToDsl, RunQueryDsl};
use serde::{Deserialize, Serialize};

use db::models::{Game, Round};
use errors::Error;

#[derive(Deserialize, Serialize)]
pub struct StatusResponse {
    pub slug: String,
    pub open_round: bool,
    pub unfinished_round: bool,
}

pub async fn get_game_status(
    connection: PooledConnection<ConnectionManager<PgConnection>>,
    game_id: i32,
) -> Result<StatusResponse, Error> {
    let data: Result<(Game, Vec<Round>), Error> = block(move || {
        let game = Game::find_by_id(&connection, game_id)?;
        let rounds = Round::belonging_to(&game).load::<Round>(&connection)?;
        Ok((game, rounds))
    })
    .await?;

    let (game, rounds) = data?;

    Ok(StatusResponse {
        slug: game.slug.unwrap_or_else(|| "".to_string()),
        open_round: rounds.iter().fold(false, |result: bool, round: &Round| {
            // if there is a round that's not locked, we want to return true
            if !round.locked {
                return true;
            }
            result
        }),
        unfinished_round: rounds.iter().fold(false, |result: bool, round: &Round| {
            if !round.finished {
                return true;
            }
            result
        }),
    })
}
