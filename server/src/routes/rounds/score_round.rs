use std::collections::HashMap;

use actix_identity::Identity;
use actix_web::web::{block, Data, HttpResponse, Json};
use serde::{Deserialize, Serialize};

use auth::{get_claim_from_identity, Role};
use db::{
    get_conn,
    models::{Round, User, UserQuestion},
    PgPool,
};
use errors::Error;

#[derive(Deserialize, Serialize)]
pub struct Answer {
    answer: String,
    question_id: i32,
}

#[derive(Deserialize, Serialize)]
pub struct Params {
    pub answers: Vec<Answer>,
}

pub async fn score_round(
    id: Identity,
    pool: Data<PgPool>,
    params: Json<Params>,
) -> Result<HttpResponse, Error> {
    let (claim, _) = get_claim_from_identity(id)?;

    if claim.role != Role::Owner {
        return Err(Error::Forbidden);
    }

    block(move || {
        let conn = get_conn(&pool)?;

        let round = Round::get_unfinished_round_by_game_id(&conn, claim.game_id)?;
        let user_questions = UserQuestion::find_by_round(&conn, round.id)?;

        let mut scores: HashMap<i32, i32> = HashMap::new();

        for uq in &user_questions {
            for answer in &params.answers {
                if answer.question_id == uq.question_id && answer.answer == uq.answer {
                    let score = {
                        let s = scores.get(&uq.user_id).unwrap_or(&0);
                        *s
                    };
                    scores.insert(uq.user_id, score + 1);
                }
            }
        }

        for (user_id, amount) in &scores {
            User::add_score(&conn, *user_id, *amount)?;
        }

        Round::finish(&conn, round.id)?;

        Ok(())
    })
    .await?;

    Ok(HttpResponse::Ok().json(()))
}

#[cfg(test)]
mod tests {
    use diesel::{self, ExpressionMethods, PgConnection, QueryDsl, RunQueryDsl};

    use db::{
        get_conn,
        models::{Game, NewUserQuestion, Question, Round, User},
        new_pool,
        schema::{games, questions as questions_dsl, rounds, user_questions, users},
    };

    use auth::{create_jwt, PrivateClaim, Role};
    use errors::ErrorResponse;

    use crate::tests::helpers::tests::test_post;

    use super::{Answer, Params};

    #[derive(Insertable)]
    #[table_name = "games"]
    struct NewGame {
        slug: Option<String>,
    }

    #[derive(Insertable)]
    #[table_name = "rounds"]
    pub struct NewRoundWithFlags {
        pub player_one: String,
        pub player_two: String,
        pub game_id: i32,
        pub locked: bool,
        pub finished: bool,
    }

    #[derive(Insertable)]
    #[table_name = "users"]
    pub struct NewUser {
        pub user_name: String,
        pub game_id: i32,
        pub score: i32,
    }

    fn create_data(conn: &PgConnection) -> (Game, Vec<Question>, Round, User) {
        let game: Game = diesel::insert_into(games::table)
            .values(NewGame {
                slug: Some("abc123".to_string()),
            })
            .get_result(conn)
            .unwrap();

        let questions: Vec<Question> = diesel::insert_into(questions_dsl::table)
            .values(&vec![
                questions_dsl::body.eq("One question".to_string()),
                questions_dsl::body.eq("Second question".to_string()),
            ])
            .get_results(conn)
            .unwrap();

        let round: Round = diesel::insert_into(rounds::table)
            .values(NewRoundWithFlags {
                player_one: "one".to_string(),
                player_two: "two".to_string(),
                game_id: game.id,
                locked: true,
                finished: false,
            })
            .get_result(conn)
            .unwrap();

        let user: User = diesel::insert_into(users::table)
            .values(NewUser {
                user_name: "agmcleod".to_string(),
                game_id: game.id,
                score: 4,
            })
            .get_result(conn)
            .unwrap();

        diesel::insert_into(user_questions::table)
            .values(vec![
                NewUserQuestion {
                    question_id: questions[0].id,
                    round_id: round.id,
                    answer: "one".to_string(),
                    user_id: user.id,
                },
                NewUserQuestion {
                    question_id: questions[1].id,
                    round_id: round.id,
                    answer: "one".to_string(),
                    user_id: user.id,
                },
            ])
            .execute(conn)
            .unwrap();

        (game, questions, round, user)
    }

    fn delete_data(conn: &PgConnection) {
        diesel::delete(user_questions::table).execute(conn).unwrap();
        diesel::delete(rounds::table).execute(conn).unwrap();
        diesel::delete(users::table).execute(conn).unwrap();
        diesel::delete(games::table).execute(conn).unwrap();
        diesel::delete(questions_dsl::table).execute(conn).unwrap();
    }

    #[actix_rt::test]
    async fn test_scoring_round_sums_amounts() {
        let pool = new_pool();
        let conn = get_conn(&pool).unwrap();

        let (game, questions, round, user) = create_data(&conn);

        let claim = PrivateClaim::new(game.id, game.slug.unwrap().clone(), game.id, Role::Owner);

        let (status, _): (u16, ()) = test_post(
            "/api/rounds/score",
            Params {
                answers: vec![
                    Answer {
                        answer: "one".to_string(),
                        question_id: questions[0].id,
                    },
                    Answer {
                        answer: "two".to_string(),
                        question_id: questions[1].id,
                    },
                ],
            },
            Some(create_jwt(claim).unwrap()),
        )
        .await;

        assert_eq!(status, 200);

        let updated_user: User = users::dsl::users.find(user.id).first(&conn).unwrap();
        assert_eq!(updated_user.score, 5);

        let updated_round: Round = rounds::dsl::rounds.find(round.id).first(&conn).unwrap();
        assert_eq!(updated_round.finished, true);

        delete_data(&conn);
    }

    #[actix_rt::test]
    async fn test_scoring_finished_rounds_returns_404() {
        let pool = new_pool();
        let conn = get_conn(&pool).unwrap();

        let (game, questions, round, _) = create_data(&conn);

        diesel::update(rounds::dsl::rounds.find(round.id))
            .set(rounds::dsl::finished.eq(true))
            .execute(&conn)
            .unwrap();

        let claim = PrivateClaim::new(game.id, game.slug.unwrap().clone(), game.id, Role::Owner);

        let (status, _): (u16, ErrorResponse) = test_post(
            "/api/rounds/score",
            Params {
                answers: vec![
                    Answer {
                        answer: "one".to_string(),
                        question_id: questions[0].id,
                    },
                    Answer {
                        answer: "two".to_string(),
                        question_id: questions[1].id,
                    },
                ],
            },
            Some(create_jwt(claim).unwrap()),
        )
        .await;

        assert_eq!(status, 404);

        delete_data(&conn);
    }
}
