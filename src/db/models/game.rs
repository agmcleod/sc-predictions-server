use chrono::{DateTime, Utc};
use diesel::{self, result, RunQueryDsl};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::db::PgConnection;
use crate::utils::create_slug_from_id;

#[derive(Debug, Serialize, Deserialize, Queryable)]
pub struct Game {
    pub id: i32,
    pub creator: Uuid,
    pub slug: String,
    pub locked: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Game {
    pub fn create(conn: &PgConnection) -> Result<Game, result::Error> {
        use crate::schema::games::{
            dsl::{games, slug},
            table,
        };

        let game: Game = diesel::insert_into(table)
            .default_values()
            .get_result(conn)?;
        let new_slug = create_slug_from_id(game.id);
        diesel::update(games::dsl::games.find(game.id))
            .set(slug.eq(new_slug))
            .get_result::<Game>(conn)
    }

    pub fn find_by_slug(conn: &PgConnection, slug_value: String) -> Result<Game, result::Error> {
        use crate::schema::games::dsl::{games, slug};

        games.filter(slug.eq(slug_value)).first::<Game>(conn)?
    }
}
