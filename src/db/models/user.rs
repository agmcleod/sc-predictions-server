use chrono::{DateTime, Utc};
use diesel::{ExpressionMethods, PgConnection, QueryDsl, RunQueryDsl};
use serde::{Deserialize, Serialize};

use crate::auth::{create_jwt, PrivateClaim};
use crate::errors::Error;
use crate::schema::users;

#[derive(Debug, Queryable, Serialize, Deserialize)]
pub struct User {
    pub id: i32,
    pub user_name: String,
    pub game_id: i32,
    pub session_id: Option<String>,
    pub score: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Insertable)]
#[table_name = "users"]
struct NewUser {
    user_name: String,
    game_id: i32,
}

impl User {
    pub fn create(
        connection: &PgConnection,
        user_name: String,
        game_id: i32,
    ) -> Result<User, Error> {
        use crate::schema::users::{dsl, table};

        let result: User = diesel::insert_into(table)
            .values(NewUser { user_name, game_id })
            .get_result(connection)?;

        let jwt = create_jwt(PrivateClaim::new(result.id, result.user_name, game_id))?;
        let result: User = diesel::update(table)
            .set(dsl::session_id.eq(jwt))
            .get_result(connection)?;

        Ok(result)
    }

    pub fn find_all_by_game_id(
        connection: &PgConnection,
        game_id: i32,
    ) -> Result<Vec<User>, Error> {
        use crate::schema::users::dsl::{game_id as gi, users};

        let results = users.filter(gi.eq(game_id)).load::<User>(connection)?;

        Ok(results)
    }

    pub fn find_by_game_id_and_name(
        connection: &PgConnection,
        game_id: i32,
        user_name: &String,
    ) -> Result<User, Error> {
        use crate::schema::users::dsl::{game_id as gi, user_name as un, users};

        let user: User = users
            .filter(un.eq(user_name))
            .filter(gi.eq(game_id))
            .first::<User>(connection)?;

        Ok(user)
    }
}
