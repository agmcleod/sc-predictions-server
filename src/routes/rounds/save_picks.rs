use std::collections::HashSet;

use actix_identity::Identity;
use actix_web::{
    web::{block, Data, HttpResponse, Json},
    Result,
};
use diesel::PgConnection;
use serde::{Deserialize, Serialize};

use crate::auth::{get_claim_from_identity, PrivateClaim, Role};
use crate::db::{
    get_conn,
    models::{GameQuestion, QuestionDetails, Round, UserQuestion},
    PgPool,
};
use crate::errors::Error;

#[derive(Deserialize, Serialize)]
pub struct Answer {
    id: i32,
    value: String,
}

#[derive(Deserialize, Serialize)]
pub struct SavePicksParams {
    answers: Vec<Answer>,
}

fn validate_user_has_not_picked(
    conn: &PgConnection,
    claim: &PrivateClaim,
    round_id: i32,
) -> Result<(), Error> {
    let results = UserQuestion::find_by_round_and_user(conn, round_id, claim.id)?;
    if (results.len() > 0) {
        return Err(Error::BadRequest(
            "User has already chosen picks for this round".to_string(),
        ));
    }

    Ok(())
}

fn validate_selected_questions(
    conn: &PgConnection,
    claim: &PrivateClaim,
    params: &Json<SavePicksParams>,
) -> Result<(), Error> {
    let questions = GameQuestion::get_questions_by_game_id(&conn, claim.game_id)?;
    let mut question_ids = HashSet::<i32>::new();
    for question in &questions {
        question_ids.insert(question.id);
    }
    // check if any answers map to questions not in this game
    for answer in &params.answers {
        if !question_ids.contains(&answer.id) {
            return Err(Error::BadRequest(format!(
                "Invalid question id: {}",
                answer.id
            )));
        }
    }

    Ok(())
}

pub async fn save_picks(
    id: Identity,
    pool: Data<PgPool>,
    params: Json<SavePicksParams>,
) -> Result<HttpResponse, Error> {
    let (claim, _) = get_claim_from_identity(id)?;

    if claim.role != Role::Player {
        return Err(Error::Forbidden);
    }

    block(move || {
        let conn = get_conn(&pool)?;

        let round = Round::get_active_round_by_game_id(&conn, claim.game_id)?;
        validate_user_has_not_picked(&conn, &claim, round.id)?;
        validate_selected_questions(&conn, &claim, &params)?;

        for answer in &params.answers {
            UserQuestion::create(&conn, claim.id, answer.id, round.id, answer.value.clone())?;
        }

        Ok(())
    })
    .await?;
    Ok(HttpResponse::Ok().json(()))
}

#[cfg(test)]
mod tests {
    use diesel::{self, ExpressionMethods, QueryDsl, RunQueryDsl};
    use serde::Serialize;

    use crate::auth::{create_jwt, PrivateClaim, Role};
    use crate::db::{
        get_conn,
        models::{Game, NewGameQuestion, NewRound, NewUser, Question, User, UserQuestion},
        new_pool,
    };
    use crate::schema::{
        game_questions, games, questions as questions_dsl, rounds, user_questions, users,
    };
    use crate::tests::helpers::tests::test_post;

    use super::{Answer, SavePicksParams};

    #[derive(Serialize, Insertable)]
    #[table_name = "games"]
    struct NewGame {
        pub locked: bool,
        pub slug: Option<String>,
    }

    #[actix_rt::test]
    async fn test_can_save_picks() {
        let pool = new_pool();
        let conn = get_conn(&pool).unwrap();

        let questions: Vec<Question> = diesel::insert_into(questions_dsl::table)
            .values(&vec![
                questions_dsl::body.eq("One question".to_string()),
                questions_dsl::body.eq("Second question".to_string()),
            ])
            .get_results(&conn)
            .unwrap();

        let game: Game = diesel::insert_into(games::table)
            .values(NewGame {
                locked: false,
                slug: None,
            })
            .get_result(&conn)
            .unwrap();

        diesel::insert_into(game_questions::table)
            .values(
                questions
                    .iter()
                    .map(|q| NewGameQuestion {
                        game_id: game.id,
                        question_id: q.id,
                    })
                    .collect::<Vec<NewGameQuestion>>(),
            )
            .execute(&conn)
            .unwrap();

        let user: User = diesel::insert_into(users::table)
            .values(NewUser {
                user_name: "agmcleod".to_string(),
                game_id: game.id,
            })
            .get_result(&conn)
            .unwrap();

        diesel::insert_into(rounds::table)
            .values(NewRound {
                player_one: "one".to_string(),
                player_two: "two".to_string(),
                game_id: game.id,
            })
            .execute(&conn)
            .unwrap();

        let claim = PrivateClaim::new(user.id, user.user_name.clone(), game.id, Role::Player);
        let token = create_jwt(claim).unwrap();

        let (status, ()) = test_post(
            "/api/rounds/set-picks",
            SavePicksParams {
                answers: vec![
                    Answer {
                        id: questions[0].id,
                        value: "one".to_string(),
                    },
                    Answer {
                        id: questions[1].id,
                        value: "two".to_string(),
                    },
                ],
            },
            Some(token),
        )
        .await;

        assert_eq!(status, 200);

        let answers: Vec<UserQuestion> = user_questions::dsl::user_questions
            .filter(user_questions::dsl::user_id.eq(user.id))
            .get_results(&conn)
            .unwrap();

        assert_eq!(answers.len(), 2);

        diesel::delete(user_questions::table)
            .execute(&conn)
            .unwrap();
        diesel::delete(rounds::table).execute(&conn).unwrap();
        diesel::delete(users::table).execute(&conn).unwrap();
        diesel::delete(game_questions::table)
            .execute(&conn)
            .unwrap();
        diesel::delete(games::table).execute(&conn).unwrap();
        diesel::delete(questions_dsl::table).execute(&conn).unwrap();
    }
}
