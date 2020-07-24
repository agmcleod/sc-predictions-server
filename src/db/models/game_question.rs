use chrono::{DateTime, Utc};
use diesel::{result::Error, PgConnection, RunQueryDsl};
use serde::{Deserialize, Serialize};

use crate::schema::game_questions;

#[derive(Debug, Serialize, Deserialize, Queryable)]
pub struct GameQuestion {
    pub id: i32,
    pub game_id: i32,
    pub question_id: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Insertable)]
#[table_name = "game_questions"]
struct NewGameQuestion {
    game_id: i32,
    question_id: i32,
}

impl GameQuestion {
    pub fn create(
        conn: &PgConnection,
        game_id: i32,
        question_id: i32,
    ) -> Result<GameQuestion, Error> {
        use crate::schema::game_questions::table;
        diesel::insert_into(table)
            .values(NewGameQuestion {
                game_id,
                question_id,
            })
            .get_result(conn)
    }
}
