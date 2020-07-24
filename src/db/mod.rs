use diesel::pg::PgConnection;
use diesel::r2d2::{ConnectionManager, Error, Pool, PooledConnection};
use r2d2_postgres::PostgresConnectionManager;

pub type PgPool = Pool<PostgresConnectionManager>;
pub mod models;

pub fn get_conn(pool: &PgPool) -> Result<PooledConnection<PostgresConnectionManager>, Error> {
    pool.get().map_err(|err| {
        error!("Failed to get connection - {}", err.to_string());
        err.into()
    })
}

pub fn new_pool(database_url: String) -> PgPool {
    let manager = ConnectionManager::<PgConnection>::new(database_url);

    Pool::builder()
        .build(manager)
        .expect("failed to create db pool")
}
