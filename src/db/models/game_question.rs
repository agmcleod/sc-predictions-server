use chrono::{DateTime, Utc};

use db::PgConnection;
use postgres_mapper;

#[derive(Debug, Serialize, Deserialize, PostgresMapper)]
pub struct GameQuestion {
    pub id: i32,
    pub game_id: i32,
    pub question_id: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl GameQuestion {
    pub fn create(conn: &PgConnection, game_id: i32, question_id: i32) -> Result<GameQuestion, postgres_mapper::Error> {
        use postgres_mapper::FromPostgresRow;

        let sql = "INSERT INTO game_questions (id, game_id, question_id) VALUES(DEFAULT, $1, $2) RETURNING *";
        let rows = conn.query(sql, &[&game_id, &question_id]).unwrap();

        GameQuestion::from_postgres_row(rows.get(0))
    }
}
