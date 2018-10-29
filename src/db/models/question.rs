use chrono::NaiveDateTime;

use db::PgConnection;
use postgres_mapper;

#[derive(Serialize, Deserialize, PostgresMapper)]
pub struct Question {
    pub id: i32,
    pub body: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

impl Question {
    pub fn get_all(conn: &PgConnection) -> Result<Vec<Question>, postgres_mapper::Error> {
        use postgres_mapper::FromPostgresRow;
        let sql = "SELECT * FROM questions";
        conn.query(sql, &[]).unwrap().into_iter().map(|row| Question::from_postgres_row(row)).collect::<Result<Vec<Question>, postgres_mapper::Error>>()
    }
}
