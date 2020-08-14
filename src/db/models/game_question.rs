use chrono::{DateTime, Utc};
use diesel::{ExpressionMethods, PgConnection, QueryDsl, RunQueryDsl};
use serde::{Deserialize, Serialize};

use crate::db::models::{Game, Question, QuestionDetails};
use crate::errors::Error;
use crate::schema::game_questions;

#[derive(Associations, Debug, Identifiable, Serialize, Deserialize, Queryable)]
#[belongs_to(Game)]
#[belongs_to(Question)]
pub struct GameQuestion {
    pub id: i32,
    pub game_id: i32,
    pub question_id: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Insertable)]
#[table_name = "game_questions"]
pub struct NewGameQuestion {
    pub game_id: i32,
    pub question_id: i32,
}

impl GameQuestion {
    pub fn create(
        conn: &PgConnection,
        game_id: i32,
        question_id: i32,
    ) -> Result<GameQuestion, Error> {
        use crate::schema::game_questions::table;
        let game_question = diesel::insert_into(table)
            .values(NewGameQuestion {
                game_id,
                question_id,
            })
            .get_result(conn)?;

        Ok(game_question)
    }

    pub fn get_questions_by_game_id(
        conn: &PgConnection,
        game_id: i32,
    ) -> Result<Vec<QuestionDetails>, Error> {
        use crate::schema::questions;

        let question_results = game_questions::dsl::game_questions
            .inner_join(questions::dsl::questions)
            .filter(game_questions::dsl::game_id.eq(game_id))
            .select((questions::dsl::id, questions::dsl::body))
            .get_results::<QuestionDetails>(conn)?;

        Ok(question_results)
    }
}
