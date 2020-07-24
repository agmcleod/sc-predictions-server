use std::env;

use diesel::pg::PgConnection;
use diesel::r2d2::{ConnectionManager, Pool, PooledConnection};
use r2d2::Error;

pub type PgPool = Pool<ConnectionManager<PgConnection>>;
pub mod models;

pub fn get_conn(pool: &PgPool) -> Result<PooledConnection<ConnectionManager<PgConnection>>, Error> {
    pool.get().map_err(|err| {
        error!("Failed to get connection - {}", err.to_string());
        err.into()
    })
}

pub fn new_pool() -> PgPool {
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let manager = ConnectionManager::<PgConnection>::new(database_url);

    Pool::builder()
        .build(manager)
        .expect("failed to create db pool")
}
