use chrono::{DateTime, Utc};
use uuid::Uuid;

use db::PgConnection;
use postgres_mapper;
use utils::create_slug_from_id;

#[derive(Debug, Serialize, Deserialize, PostgresMapper)]
pub struct Game {
    pub id: i32,
    pub creator: Uuid,
    pub slug: String,
    pub locked: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Game {
    pub fn create(conn: &PgConnection) -> Result<Game, postgres_mapper::Error> {
        use postgres_mapper::FromPostgresRow;
        let sql = "INSERT INTO games (id) VALUES(default) RETURNING *";
        let rows = conn.query(sql, &[]).unwrap();
        let id: i32 = rows.get(0).get("id");
        let slug = create_slug_from_id(id);
        let sql = "UPDATE games SET slug = $1 WHERE id = $2 RETURNING *";
        let rows = conn.query(sql, &[&slug, &id]).unwrap();
        Game::from_postgres_row(rows.get(0))
    }
}
