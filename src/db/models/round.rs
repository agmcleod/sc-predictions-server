use chrono::{DateTime, Utc};
use diesel::{self, PgConnection, RunQueryDsl};
use serde::{Deserialize, Serialize};

use crate::db::models::Game;
use crate::errors::Error;
use crate::schema::rounds::{self, table};

#[derive(Associations, Debug, Deserialize, Identifiable, Serialize, Queryable)]
#[belongs_to(Game)]
pub struct Round {
    pub id: i32,
    pub player_one: String,
    pub player_two: String,
    pub game_id: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub locked: bool,
}

#[derive(Insertable)]
#[table_name = "rounds"]
pub struct NewRound {
    pub player_one: String,
    pub player_two: String,
    pub game_id: i32,
}

impl Round {
    pub fn create(
        conn: &PgConnection,
        game_id: i32,
        player_one: String,
        player_two: String,
    ) -> Result<Round, Error> {
        let round = diesel::insert_into(table)
            .values(NewRound {
                player_one,
                player_two,
                game_id,
            })
            .get_result(conn)?;

        Ok(round)
    }
}
