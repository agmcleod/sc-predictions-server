use actix_web::{error, Error};
use chrono::{DateTime, Utc};
use postgres::transaction::Transaction;
use uuid::Uuid;

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
    pub fn create(transaction: &Transaction) -> Result<Game, Error> {
        use postgres_mapper::FromPostgresRow;
        let sql = "INSERT INTO games (id) VALUES(default) RETURNING *";
        transaction
            .query(sql, &[])
            .and_then(|rows| {
                let id: i32 = rows.get(0).get("id");
                let slug = create_slug_from_id(id);
                let sql = "UPDATE games SET slug = $1 WHERE id = $2 RETURNING *";
                transaction.query(sql, &[&slug, &id])
            })
            .map_err(|err| error::ErrorInternalServerError(err))
            .and_then(|rows| {
                Game::from_postgres_row(rows.get(0))
                    .map_err(|err| error::ErrorInternalServerError(err))
            })
    }
}
