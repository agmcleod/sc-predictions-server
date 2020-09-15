use actix_identity::Identity;
use actix_web::web::{block, Data, Json};
use serde::{Deserialize, Serialize};

use auth::{get_claim_from_identity, Role};
use db::{
    get_conn,
    models::{Round, UserAnswer, UserQuestion},
    PgPool,
};
use errors::Error;

#[derive(Deserialize, PartialEq, Serialize)]
pub struct GetRoundPicksResponse {
    data: Vec<UserAnswer>,
}

pub async fn get_round_picks(
    id: Identity,
    pool: Data<PgPool>,
) -> Result<Json<GetRoundPicksResponse>, Error> {
    let (claim, _) = get_claim_from_identity(id)?;
    if claim.role != Role::Owner {
        return Err(Error::Forbidden);
    }

    let user_questions = block(move || {
        let conn = get_conn(&pool)?;

        let round = Round::get_active_round_by_game_id(&conn, claim.game_id)?;

        let user_questions = UserQuestion::find_by_round(&conn, round.id)?;
        Ok(user_questions)
    })
    .await?;

    Ok(Json(GetRoundPicksResponse {
        data: user_questions,
    }))
}
