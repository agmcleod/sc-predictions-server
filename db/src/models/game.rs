use chrono::{DateTime, Utc};
use diesel::{self, ExpressionMethods, PgConnection, QueryDsl, RunQueryDsl};
use serde::{Deserialize, Serialize};

use auth::{create_jwt, PrivateClaim, Role};
use errors::Error;

use crate::schema::games;
use crate::utils::create_slug_from_id;

#[derive(Debug, Identifiable, Serialize, Deserialize, Queryable)]
pub struct Game {
    pub id: i32,
    pub slug: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub creator: Option<String>,
}

impl Game {
    pub fn create(conn: &PgConnection) -> Result<Game, Error> {
        use games::{dsl, table};

        let game: Game = diesel::insert_into(table)
            .default_values()
            .get_result(conn)?;
        let new_slug = create_slug_from_id(game.id);
        let jwt = create_jwt(PrivateClaim::new(
            game.id,
            new_slug.clone(),
            game.id,
            Role::Owner,
        ))?;
        let updated_game = diesel::update(dsl::games.find(game.id))
            .set((dsl::slug.eq(new_slug), dsl::creator.eq(jwt)))
            .get_result::<Game>(conn)?;

        Ok(updated_game)
    }

    pub fn find_by_id(conn: &PgConnection, id: i32) -> Result<Game, Error> {
        use crate::schema::games::dsl::games;

        let game = games.find(id).first(conn)?;

        Ok(game)
    }

    pub fn find_by_slug(conn: &PgConnection, slug_value: &str) -> Result<Game, Error> {
        use crate::schema::games::dsl::{games, slug};

        let game = games.filter(slug.eq(slug_value)).first::<Game>(conn)?;

        Ok(game)
    }
}
