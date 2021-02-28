use actix_web::web::block;
use diesel::pg::PgConnection;
use diesel::r2d2::{ConnectionManager, PooledConnection};
use serde::{Deserialize, Serialize};

use db::models::{GameQuestion, QuestionDetails, Round};
use errors::Error;

#[derive(Deserialize, PartialEq, Serialize)]
pub struct RoundStatusRepsonse {
    pub player_names: Vec<String>,
    pub questions: Vec<QuestionDetails>,
    pub round_id: i32,
    pub locked: bool,
    pub finished: bool,
}

pub async fn get_round_status(
    connection: PooledConnection<ConnectionManager<PgConnection>>,
    game_id: i32,
) -> Result<RoundStatusRepsonse, Error> {
    let (round, questions) = block(move || {
        let round = Round::get_latest_round_by_game_id(&connection, game_id)?;
        let questions = GameQuestion::get_questions_by_game_id(&connection, game_id)?;

        Ok((round, questions))
    })
    .await?;

    Ok(RoundStatusRepsonse {
        player_names: vec![round.player_one, round.player_two],
        questions,
        round_id: round.id,
        locked: round.locked,
        finished: round.finished,
    })
}
