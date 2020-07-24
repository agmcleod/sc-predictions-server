use actix_web::error;
use chrono::{DateTime, Utc};
use diesel::PgConnection;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::schema::users;
use crate::errors::{Error};

#[derive(Debug, Serialize, Deserialize)]
pub struct User {
    pub id: i32,
    pub game_id: i32,
    pub user_name: String,
    pub session_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Insertable)]
#[table_name="users"]
pub struct NewUser {
    user_name: String,
    game_id: i32,
}

impl User {
    pub fn create(connection: &PgConnection, user_name: String, game_id: i32) -> Result<User, Error> {
        use crate::schema::users::{table};

        diesel::insert_into(table).values(NewUser { user_name, game_id }).map_err(|err| error::ErrorInternalServerError(err))
    }
}
