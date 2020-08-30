use chrono::{DateTime, Utc};
use diesel::{self, ExpressionMethods, PgConnection, QueryDsl, RunQueryDsl};
use serde::{Deserialize, Serialize};

use errors::Error;

use crate::models::{Question, Round, User};
use crate::schema::user_questions;

#[derive(Associations, Deserialize, Queryable, Identifiable, Serialize)]
#[table_name = "user_questions"]
#[belongs_to(Round)]
#[belongs_to(User)]
#[belongs_to(Question)]
pub struct UserQuestion {
    pub id: i32,
    pub user_id: i32,
    pub question_id: i32,
    pub round_id: i32,
    pub answer: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Insertable, Serialize)]
#[table_name = "user_questions"]
pub struct NewUserQuestion {
    pub user_id: i32,
    pub question_id: i32,
    pub round_id: i32,
    pub answer: String,
}

impl UserQuestion {
    pub fn create(
        conn: &PgConnection,
        user_id: i32,
        question_id: i32,
        round_id: i32,
        answer: String,
    ) -> Result<UserQuestion, Error> {
        let user_question = diesel::insert_into(user_questions::table)
            .values(NewUserQuestion {
                user_id,
                question_id,
                round_id,
                answer,
            })
            .get_result(conn)?;

        Ok(user_question)
    }

    pub fn find_by_round_and_user(
        conn: &PgConnection,
        round_id: i32,
        user_id: i32,
    ) -> Result<Vec<UserQuestion>, Error> {
        use user_questions::dsl::{
            round_id as round_id_dsl, user_id as user_id_dsl,
            user_questions as user_questions_table,
        };

        let results = user_questions_table
            .filter(round_id_dsl.eq(round_id))
            .filter(user_id_dsl.eq(user_id))
            .get_results(conn)?;

        Ok(results)
    }
}
