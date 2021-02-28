use actix_identity::Identity;
use actix_web::{
    web::{Data, Json},
    Result,
};

use auth::get_claim_from_identity;
use db::{get_conn, PgPool};
use errors::Error;

use crate::handlers::{get_round_status, RoundStatusRepsonse};

pub async fn status(id: Identity, pool: Data<PgPool>) -> Result<Json<RoundStatusRepsonse>, Error> {
    let (claim, _) = get_claim_from_identity(id)?;
    let conn = get_conn(&pool)?;
    let status = get_round_status(conn, claim.role, claim.id, claim.game_id).await?;
    Ok(Json(status))
}

#[cfg(test)]
mod tests {
    use diesel::{self, RunQueryDsl};

    use super::RoundStatusRepsonse;
    use crate::tests::helpers::tests::test_get;
    use auth::{create_jwt, PrivateClaim, Role};
    use db::{
        get_conn,
        models::{
            Game, NewGameQuestion, NewUser, NewUserQuestion, Question, QuestionDetails, Round, User,
        },
        new_pool,
        schema::{game_questions, games, questions, rounds, user_questions, users},
    };

    #[derive(Insertable)]
    #[table_name = "games"]
    struct NewGame {
        slug: Option<String>,
    }

    #[derive(Insertable)]
    #[table_name = "questions"]
    struct NewQuestion {
        body: String,
    }

    #[derive(Insertable)]
    #[table_name = "rounds"]
    pub struct NewRound {
        pub player_one: String,
        pub player_two: String,
        pub game_id: i32,
        pub locked: bool,
    }

    #[actix_rt::test]
    async fn test_status_get_player_names_questions() {
        let pool = new_pool();
        let conn = get_conn(&pool).unwrap();

        let question_one: Question = diesel::insert_into(questions::table)
            .values(NewQuestion {
                body: "Who will expand first?".to_string(),
            })
            .get_result(&conn)
            .unwrap();
        let question_two: Question = diesel::insert_into(questions::table)
            .values(NewQuestion {
                body: "Who will strike first?".to_string(),
            })
            .get_result(&conn)
            .unwrap();

        let game: Game = diesel::insert_into(games::table)
            .values(NewGame {
                slug: Some("abc123".to_string()),
            })
            .get_result(&conn)
            .unwrap();

        let round: Round = diesel::insert_into(rounds::table)
            .values(NewRound {
                player_one: "one".to_string(),
                player_two: "two".to_string(),
                game_id: game.id,
                locked: false,
            })
            .get_result(&conn)
            .unwrap();

        diesel::insert_into(game_questions::table)
            .values(NewGameQuestion {
                game_id: game.id,
                question_id: question_one.id,
            })
            .execute(&conn)
            .unwrap();
        diesel::insert_into(game_questions::table)
            .values(NewGameQuestion {
                game_id: game.id,
                question_id: question_two.id,
            })
            .execute(&conn)
            .unwrap();

        let claim = PrivateClaim::new(game.id, game.slug.unwrap().clone(), game.id, Role::Owner);
        let token = create_jwt(claim).unwrap();
        let res: (u16, RoundStatusRepsonse) = test_get("/api/current-round", Some(token)).await;

        assert_eq!(res.0, 200);
        assert_eq!(res.1.player_names, vec!["one", "two"]);
        assert_eq!(
            res.1.questions,
            vec![
                QuestionDetails {
                    id: question_one.id,
                    body: question_one.body
                },
                QuestionDetails {
                    id: question_two.id,
                    body: question_two.body
                }
            ]
        );
        assert_eq!(res.1.round_id, round.id);
        assert_eq!(res.1.locked, false);
        assert_eq!(res.1.finished, false);

        diesel::delete(game_questions::table)
            .execute(&conn)
            .unwrap();
        diesel::delete(questions::table).execute(&conn).unwrap();
        diesel::delete(rounds::table).execute(&conn).unwrap();
        diesel::delete(games::table).execute(&conn).unwrap();
    }

    #[actix_rt::test]
    async fn test_status_player_has_picks() {
        let pool = new_pool();
        let conn = get_conn(&pool).unwrap();

        let question_one: Question = diesel::insert_into(questions::table)
            .values(NewQuestion {
                body: "Who will expand first?".to_string(),
            })
            .get_result(&conn)
            .unwrap();
        let question_two: Question = diesel::insert_into(questions::table)
            .values(NewQuestion {
                body: "Who will strike first?".to_string(),
            })
            .get_result(&conn)
            .unwrap();

        let game: Game = diesel::insert_into(games::table)
            .values(NewGame {
                slug: Some("abc123".to_string()),
            })
            .get_result(&conn)
            .unwrap();

        let round: Round = diesel::insert_into(rounds::table)
            .values(NewRound {
                player_one: "one".to_string(),
                player_two: "two".to_string(),
                game_id: game.id,
                locked: false,
            })
            .get_result(&conn)
            .unwrap();

        diesel::insert_into(game_questions::table)
            .values(NewGameQuestion {
                game_id: game.id,
                question_id: question_one.id,
            })
            .execute(&conn)
            .unwrap();
        diesel::insert_into(game_questions::table)
            .values(NewGameQuestion {
                game_id: game.id,
                question_id: question_two.id,
            })
            .execute(&conn)
            .unwrap();

        let user: User = diesel::insert_into(users::table)
            .values(NewUser {
                game_id: game.id,
                user_name: "agmcleod".to_string(),
            })
            .get_result(&conn)
            .unwrap();

        diesel::insert_into(user_questions::table)
            .values(vec![
                NewUserQuestion {
                    answer: "one".to_string(),
                    question_id: question_one.id,
                    round_id: round.id,
                    user_id: user.id,
                },
                NewUserQuestion {
                    answer: "one".to_string(),
                    question_id: question_two.id,
                    round_id: round.id,
                    user_id: user.id,
                },
            ])
            .execute(&conn)
            .unwrap();

        let claim = PrivateClaim::new(user.id, user.user_name.clone(), game.id, Role::Player);
        let token = create_jwt(claim).unwrap();
        let res: (u16, RoundStatusRepsonse) = test_get("/api/current-round", Some(token)).await;

        assert_eq!(res.0, 200);
        assert_eq!(res.1.player_names, vec!["one", "two"]);
        assert_eq!(
            res.1.questions,
            vec![
                QuestionDetails {
                    id: question_one.id,
                    body: question_one.body
                },
                QuestionDetails {
                    id: question_two.id,
                    body: question_two.body
                }
            ]
        );
        assert_eq!(res.1.round_id, round.id);
        assert_eq!(res.1.locked, false);
        assert_eq!(res.1.finished, false);
        assert_eq!(res.1.picks_chosen, true);

        diesel::delete(user_questions::table)
            .execute(&conn)
            .unwrap();
        diesel::delete(users::table).execute(&conn).unwrap();
        diesel::delete(game_questions::table)
            .execute(&conn)
            .unwrap();
        diesel::delete(questions::table).execute(&conn).unwrap();
        diesel::delete(rounds::table).execute(&conn).unwrap();
        diesel::delete(games::table).execute(&conn).unwrap();
    }

    #[actix_rt::test]
    async fn test_current_round_no_active_round() {
        let pool = new_pool();
        let conn = get_conn(&pool).unwrap();

        let game: Game = diesel::insert_into(games::table)
            .values(NewGame {
                slug: Some("abc123".to_string()),
            })
            .get_result(&conn)
            .unwrap();

        diesel::insert_into(rounds::table)
            .values(NewRound {
                player_one: "boxer".to_string(),
                player_two: "idra".to_string(),
                game_id: game.id,
                locked: true,
            })
            .execute(&conn)
            .unwrap();

        diesel::insert_into(rounds::table)
            .values(NewRound {
                player_one: "mvp".to_string(),
                player_two: "mc".to_string(),
                game_id: game.id,
                locked: true,
            })
            .execute(&conn)
            .unwrap();

        let claim = PrivateClaim::new(game.id, game.slug.unwrap().clone(), game.id, Role::Owner);
        let token = create_jwt(claim).unwrap();
        let res: (u16, RoundStatusRepsonse) = test_get("/api/current-round", Some(token)).await;

        assert_eq!(res.0, 200);

        assert_eq!(res.1.locked, true);
        assert_eq!(res.1.player_names, vec!["mvp", "mc"]);

        diesel::delete(rounds::table).execute(&conn).unwrap();
        diesel::delete(games::table).execute(&conn).unwrap();
    }
}
