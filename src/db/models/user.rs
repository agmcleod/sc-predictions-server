use chrono::{DateTime, Utc};
use uuid::Uuid;

use db::PgConnection;
use errors::{DBError, Error};

#[derive(Debug, Serialize, Deserialize, PostgresMapper)]
pub struct User {
    pub id: i32,
    pub game_id: i32,
    pub user_name: String,
    pub session_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl User {
    pub fn create(connection: &PgConnection, name: String, game_id: i32) -> Result<User, Error> {
        use postgres_mapper::FromPostgresRow;
        let sql = "INSERT INTO users (user_name, game_id) VALUES ($1, $2) RETURNING *";
        connection
            .query(sql, &[&name, &game_id])
            .map_err(|err| Error::DBError(DBError::PGError(err)))
            .and_then(|rows| {
                User::from_postgres_row(rows.get(0))
                    .map_err(|err| Error::DBError(DBError::MapError(err.to_string())))
            })
    }
}
