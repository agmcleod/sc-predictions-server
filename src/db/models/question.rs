use chrono::{DateTime, Utc};
use diesel::{PgConnection, QueryDsl, RunQueryDsl};
use serde::{Deserialize, Serialize};

use crate::errors::Error;

#[derive(Debug, Serialize, Deserialize, Queryable)]
pub struct Question {
    pub id: i32,
    pub body: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Question {
    pub fn get_all(conn: &PgConnection) -> Result<Vec<Question>, Error> {
        use crate::schema::questions::dsl::{body, questions};

        let all_questions = questions.order(body).load::<Question>(conn)?;

        Ok(all_questions)
    }
}
