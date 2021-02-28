use actix_web::web::block;
use diesel::pg::PgConnection;
use diesel::r2d2::{ConnectionManager, PooledConnection};
use serde::{Deserialize, Serialize};

use auth::Role;
use db::models::{GameQuestion, QuestionDetails, Round, UserQuestion};
use errors::Error;

#[derive(Deserialize, PartialEq, Serialize)]
pub struct RoundStatusRepsonse {
    pub player_names: Vec<String>,
    pub questions: Vec<QuestionDetails>,
    pub round_id: i32,
    pub locked: bool,
    pub finished: bool,
    pub picks_chosen: bool,
}

pub async fn get_round_status(
    connection: PooledConnection<ConnectionManager<PgConnection>>,
    role: Role,
    user_id: i32,
    game_id: i32,
) -> Result<RoundStatusRepsonse, Error> {
    let (round, questions, user_questions) = block(move || {
        let round = Round::get_latest_round_by_game_id(&connection, game_id)?;
        let questions = GameQuestion::get_questions_by_game_id(&connection, game_id)?;

        let user_questions = if role == Role::Player {
            UserQuestion::find_by_round_and_user(&connection, round.id, user_id)?
        } else {
            Vec::new()
        };

        Ok((round, questions, user_questions))
    })
    .await?;

    Ok(RoundStatusRepsonse {
        player_names: vec![round.player_one, round.player_two],
        questions,
        round_id: round.id,
        locked: round.locked,
        finished: round.finished,
        picks_chosen: user_questions.len() > 0,
    })
}
