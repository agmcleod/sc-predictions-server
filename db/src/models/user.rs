use chrono::{DateTime, Utc};
use diesel::{ExpressionMethods, PgConnection, QueryDsl, RunQueryDsl};
use serde::{Deserialize, Serialize};

use auth::{create_jwt, PrivateClaim, Role};
use errors::Error;

use crate::schema::users;

#[derive(Debug, Queryable, Identifiable, Serialize, Deserialize)]
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
pub struct NewUser {
    pub user_name: String,
    pub game_id: i32,
}

#[derive(Deserialize, Identifiable, Queryable, Serialize)]
#[table_name = "users"]
pub struct UserDetails {
    pub id: i32,
    pub user_name: String,
    pub game_id: i32,
    pub score: i32,
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

        let jwt = create_jwt(PrivateClaim::new(
            result.id,
            result.user_name,
            game_id,
            Role::Player,
        ))?;
        let result: User = diesel::update(table)
            .set(dsl::session_id.eq(jwt))
            .get_result(connection)?;

        Ok(result)
    }

    pub fn find_all_by_game_id(
        connection: &PgConnection,
        game_id: i32,
    ) -> Result<Vec<UserDetails>, Error> {
        use crate::schema::users::dsl::{game_id as game_id_field, id, score, user_name, users};

        let results = users
            .select((id, user_name, game_id_field, score))
            .filter(game_id_field.eq(game_id))
            .get_results::<UserDetails>(connection)?;

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

    pub fn add_score(connection: &PgConnection, user_id: i32, amount: i32) -> Result<User, Error> {
        use crate::schema::users::dsl::{id, score as score_field, users as users_table};

        let score = users_table
            .select(score_field)
            .filter(id.eq(user_id))
            .get_result::<i32>(connection)?;

        let user = diesel::update(users_table.filter(id.eq(user_id)))
            .set(score_field.eq(score + amount))
            .get_result(connection)?;

        Ok(user)
    }
}
