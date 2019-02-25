use actix_web::{error, Error};
use chrono::{DateTime, Utc};

use db::PgConnection;

#[derive(Debug, Serialize, Deserialize, PostgresMapper)]
pub struct Question {
    pub id: i32,
    pub body: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Question {
    pub fn get_all(conn: &PgConnection) -> Result<Vec<Question>, Error> {
        use postgres_mapper::FromPostgresRow;
        let sql = "SELECT * FROM questions";
        conn.query(sql, &[])
            .unwrap()
            .into_iter()
            .map(|row| {
                Question::from_postgres_row(row).map_err(|err| error::ErrorInternalServerError(err))
            })
            .collect::<Result<Vec<Question>, Error>>()
    }
}
