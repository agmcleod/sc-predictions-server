use std::collections::HashSet;

use actix::Addr;
use actix_identity::Identity;
use actix_web::{
    web::{block, Data, Json},
    HttpResponse, Result,
};
use diesel::PgConnection;
use serde::{Deserialize, Serialize};
use serde_json::to_value;

use auth::{get_claim_from_identity, PrivateClaim, Role};
use db::{
    get_conn,
    models::{GameQuestion, Round, UserQuestion},
    PgPool,
};
use errors::Error;

use crate::handlers::get_round_picks;
use crate::websocket::{MessageToClient, Server};

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
    if results.len() > 0 {
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

    if questions.len() != params.answers.len() {
        return Err(Error::BadRequest(format!(
            "Received {} answers, expected {}",
            params.answers.len(),
            questions.len()
        )));
    }

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
    websocket_srv: Data<Addr<Server>>,
    pool: Data<PgPool>,
    params: Json<SavePicksParams>,
) -> Result<HttpResponse, Error> {
    let (claim, _) = get_claim_from_identity(id)?;

    if claim.role != Role::Player {
        return Err(Error::Forbidden);
    }

    let conn = get_conn(&pool)?;

    let res: Result<PrivateClaim, Error> = block(move || {
        let round = Round::get_active_round_by_game_id(&conn, claim.game_id)?;
        validate_user_has_not_picked(&conn, &claim, round.id)?;
        validate_selected_questions(&conn, &claim, &params)?;

        for answer in &params.answers {
            UserQuestion::create(&conn, claim.id, answer.id, round.id, answer.value.clone())?;
        }

        Ok(claim)
    })
    .await?;

    let claim = res?;

    let conn = get_conn(&pool)?;
    let round_picks = get_round_picks(conn, claim.game_id).await;
    match round_picks {
        Ok(round_picks) => {
            if let Ok(value) = to_value(round_picks) {
                let msg = MessageToClient::new("/picks", claim.game_id, value);
                websocket_srv.do_send(msg);
            }
        }
        Err(err) => error!("{:?}", err),
    }

    Ok(HttpResponse::Ok().json(()))
}

#[cfg(test)]
mod tests {
    use actix_web_actors::ws;
    use awc::Client;
    use diesel::{self, ExpressionMethods, PgConnection, QueryDsl, RunQueryDsl};
    use futures::{SinkExt, StreamExt};
    use serde::Serialize;

    use auth::{create_jwt, PrivateClaim, Role};
    use db::{
        get_conn,
        models::{
            Game, NewGameQuestion, NewRound, NewUser, NewUserQuestion, Question, Round, User,
            UserAnswer, UserQuestion,
        },
        new_pool,
        schema::{
            game_questions, games, questions as questions_dsl, rounds, user_questions, users,
        },
    };
    use errors::ErrorResponse;

    use super::{Answer, SavePicksParams};
    use crate::handlers::GetRoundPicksResponse;
    use crate::tests::helpers::tests::{get_test_server, get_websocket_frame_data, test_post};

    #[derive(Serialize, Insertable)]
    #[table_name = "games"]
    struct NewGame {
        pub slug: Option<String>,
    }

    fn create_game_data(conn: &PgConnection) -> (Vec<Question>, Game, User, Round) {
        let questions: Vec<Question> = diesel::insert_into(questions_dsl::table)
            .values(&vec![
                questions_dsl::body.eq("One question".to_string()),
                questions_dsl::body.eq("Second question".to_string()),
            ])
            .get_results(conn)
            .unwrap();

        let game: Game = diesel::insert_into(games::table)
            .values(NewGame { slug: None })
            .get_result(conn)
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

        (questions, game, user, round)
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
    async fn test_can_save_picks() {
        let pool = new_pool();
        let conn = get_conn(&pool).unwrap();

        let (questions, game, user, _) = create_game_data(&conn);

        let claim = PrivateClaim::new(user.id, user.user_name.clone(), game.id, Role::Player);
        let token = create_jwt(claim).unwrap();

        let srv = get_test_server();

        let client = Client::default();
        let mut ws_conn = client.ws(srv.url("/ws/")).connect().await.unwrap();

        ws_conn
            .1
            .send(ws::Message::Text(
                format!("/auth {{\"token\":\"{}\"}}", token).into(),
            ))
            .await
            .unwrap();

        let res = srv
            .post("/api/rounds/set-picks")
            .append_header(("Authorization", token))
            .send_json(&SavePicksParams {
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
            })
            .await
            .unwrap();

        assert_eq!(res.status().as_u16(), 200);

        let mut stream = ws_conn.1.take(2);
        // skip the first one, as it's a heartbeat
        stream.next().await;
        let msg = stream.next().await;

        let data = get_websocket_frame_data(msg.unwrap().unwrap());
        if data.is_some() {
            let msg = data.unwrap();
            assert_eq!(msg.path, "/picks");
            assert_eq!(msg.game_id, game.id);
            let round_picks: GetRoundPicksResponse = serde_json::from_value(msg.data).unwrap();
            assert_eq!(round_picks.locked, false);
            assert_eq!(round_picks.data[0].answer, "one");
            assert_eq!(round_picks.data[1].answer, "two");
            assert_eq!(round_picks.data.len(), 2);
        } else {
            assert!(false, "Message was not a string");
        }

        drop(stream);

        srv.stop().await;

        let answers: Vec<UserQuestion> = user_questions::dsl::user_questions
            .filter(user_questions::dsl::user_id.eq(user.id))
            .get_results(&conn)
            .unwrap();

        assert_eq!(answers.len(), 2);

        clear_game_data(&conn);
    }

    #[actix_rt::test]
    async fn test_owner_cannot_select() {
        let pool = new_pool();
        let conn = get_conn(&pool).unwrap();

        let (questions, game, user, _) = create_game_data(&conn);

        let claim = PrivateClaim::new(user.id, user.user_name.clone(), game.id, Role::Owner);
        let token = create_jwt(claim).unwrap();

        let (status, _): (u16, ErrorResponse) = test_post(
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

        assert_eq!(status, 403);

        clear_game_data(&conn);
    }

    #[actix_rt::test]
    async fn test_no_active_round() {
        let pool = new_pool();
        let conn = get_conn(&pool).unwrap();

        let (questions, game, user, round) = create_game_data(&conn);

        diesel::update(rounds::table)
            .set(rounds::dsl::locked.eq(true))
            .filter(rounds::dsl::id.eq(round.id))
            .execute(&conn)
            .unwrap();

        let claim = PrivateClaim::new(user.id, user.user_name.clone(), game.id, Role::Player);
        let token = create_jwt(claim).unwrap();

        let (status, _): (u16, ErrorResponse) = test_post(
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

        assert_eq!(status, 404);

        clear_game_data(&conn);
    }

    #[actix_rt::test]
    async fn test_player_has_already_picked() {
        let pool = new_pool();
        let conn = get_conn(&pool).unwrap();

        let (questions, game, user, round) = create_game_data(&conn);

        diesel::insert_into(user_questions::table)
            .values(NewUserQuestion {
                user_id: user.id,
                question_id: questions[0].id,
                round_id: round.id,
                answer: round.player_one.clone(),
            })
            .execute(&conn)
            .unwrap();

        let claim = PrivateClaim::new(user.id, user.user_name.clone(), game.id, Role::Player);
        let token = create_jwt(claim).unwrap();

        let (status, err): (u16, ErrorResponse) = test_post(
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

        assert_eq!(status, 400);
        assert_eq!(
            err.errors[0],
            "User has already chosen picks for this round"
        );

        clear_game_data(&conn);
    }

    #[actix_rt::test]
    async fn test_player_has_answered_valid_questions() {
        let pool = new_pool();
        let conn = get_conn(&pool).unwrap();

        let (questions, game, user, _) = create_game_data(&conn);

        let claim = PrivateClaim::new(user.id, user.user_name.clone(), game.id, Role::Player);
        let token = create_jwt(claim).unwrap();

        let answer_id = questions[0].id + 10;
        let (status, err): (u16, ErrorResponse) = test_post(
            "/api/rounds/set-picks",
            SavePicksParams {
                answers: vec![
                    Answer {
                        id: answer_id,
                        value: "one".to_string(),
                    },
                    Answer {
                        id: questions[1].id + 15,
                        value: "two".to_string(),
                    },
                ],
            },
            Some(token),
        )
        .await;

        assert_eq!(status, 400);
        assert_eq!(err.errors[0], format!("Invalid question id: {}", answer_id));

        clear_game_data(&conn);
    }

    #[actix_rt::test]
    async fn test_player_missed_a_question() {
        let pool = new_pool();
        let conn = get_conn(&pool).unwrap();

        let (questions, game, user, _) = create_game_data(&conn);

        let claim = PrivateClaim::new(user.id, user.user_name.clone(), game.id, Role::Player);
        let token = create_jwt(claim).unwrap();

        let (status, err): (u16, ErrorResponse) = test_post(
            "/api/rounds/set-picks",
            SavePicksParams {
                answers: vec![Answer {
                    id: questions[0].id,
                    value: "one".to_string(),
                }],
            },
            Some(token),
        )
        .await;

        assert_eq!(status, 400);
        assert_eq!(err.errors[0], "Received 1 answers, expected 2".to_string());

        clear_game_data(&conn);
    }
}
