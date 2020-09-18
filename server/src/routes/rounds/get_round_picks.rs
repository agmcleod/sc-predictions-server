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

#[cfg(test)]
mod tests {
    use diesel::{self, ExpressionMethods, PgConnection, QueryDsl, RunQueryDsl};

    use auth::{create_jwt, PrivateClaim, Role};
    use db::{
        get_conn,
        models::{
            Game, NewGameQuestion, NewRound, NewUser, NewUserQuestion, Question, Round, User,
            UserQuestion,
        },
        new_pool,
        schema::{
            game_questions, games, questions as questions_dsl, rounds, user_questions, users,
        },
        PgPool,
    };
    use errors::ErrorResponse;

    use crate::tests::helpers::tests::test_get;

    use super::GetRoundPicksResponse;

    #[derive(Insertable)]
    #[table_name = "games"]
    struct NewGame {
        slug: Option<String>,
    }

    fn create_test_data(conn: &PgConnection) -> (Game, User, Round, Vec<UserQuestion>) {
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
            .execute(conn)
            .unwrap();

        let user: User = diesel::insert_into(users::table)
            .values(NewUser {
                user_name: "agmcleod".to_string(),
                game_id: game.id,
            })
            .get_result(conn)
            .unwrap();

        let round: Round = diesel::insert_into(rounds::table)
            .values(NewRound {
                player_one: "one".to_string(),
                player_two: "two".to_string(),
                game_id: game.id,
            })
            .get_result(conn)
            .unwrap();

        let new_user_questions: Vec<UserQuestion> = diesel::insert_into(user_questions::table)
            .values(vec![
                NewUserQuestion {
                    user_id: user.id,
                    question_id: questions[0].id,
                    round_id: round.id,
                    answer: "one".to_string(),
                },
                NewUserQuestion {
                    user_id: user.id,
                    question_id: questions[1].id,
                    round_id: round.id,
                    answer: "two".to_string(),
                },
            ])
            .get_results(conn)
            .unwrap();

        (game, user, round, new_user_questions)
    }

    fn clear_game_data(conn: &PgConnection) {
        diesel::delete(user_questions::table).execute(conn).unwrap();
        diesel::delete(rounds::table).execute(conn).unwrap();
        diesel::delete(users::table).execute(conn).unwrap();
        diesel::delete(game_questions::table).execute(conn).unwrap();
        diesel::delete(games::table).execute(conn).unwrap();
        diesel::delete(questions_dsl::table).execute(conn).unwrap();
    }

    #[actix_rt::test]
    async fn test_get_round_picks() {
        let pool = new_pool();
        let conn = get_conn(&pool).unwrap();
        let (game, user, _, new_user_questions) = create_test_data(&conn);

        let claim = PrivateClaim::new(game.id, game.slug.unwrap().clone(), game.id, Role::Owner);

        let (status, body): (u16, GetRoundPicksResponse) =
            test_get("/api/rounds/picks", Some(create_jwt(claim).unwrap())).await;

        assert_eq!(status, 200);
        assert_eq!(body.data.len(), 2);
        let first_pick = &body.data[0];
        assert_eq!(first_pick.user_name, "agmcleod");
        assert_eq!(first_pick.answer, "one");

        let second_pick = &body.data[1];
        assert_eq!(second_pick.user_name, "agmcleod");
        assert_eq!(second_pick.answer, "two");

        clear_game_data(&conn);
    }

    #[actix_rt::test]
    async fn test_get_round_picks_role_not_owner() {
        let pool = new_pool();
        let conn = get_conn(&pool).unwrap();
        let (game, user, _, new_user_questions) = create_test_data(&conn);

        let claim = PrivateClaim::new(game.id, game.slug.unwrap().clone(), game.id, Role::Player);

        let (status, _): (u16, ErrorResponse) =
            test_get("/api/rounds/picks", Some(create_jwt(claim).unwrap())).await;

        assert_eq!(status, 403);

        clear_game_data(&conn);
    }

    #[actix_rt::test]
    async fn test_get_round_picks_no_active_round() {
        let pool = new_pool();
        let conn = get_conn(&pool).unwrap();
        let (game, user, round, new_user_questions) = create_test_data(&conn);

        diesel::update(rounds::dsl::rounds.find(round.id))
            .set(rounds::dsl::locked.eq(true))
            .execute(&conn)
            .unwrap();

        let claim = PrivateClaim::new(game.id, game.slug.unwrap().clone(), game.id, Role::Owner);

        let (status, _): (u16, ErrorResponse) =
            test_get("/api/rounds/picks", Some(create_jwt(claim).unwrap())).await;

        assert_eq!(status, 404);

        clear_game_data(&conn);
    }
}
