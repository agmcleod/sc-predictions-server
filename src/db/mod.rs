use actix::{Actor, SyncContext};

use r2d2::{Pool, PooledConnection};
use r2d2_postgres::{PostgresConnectionManager, TlsMode};

use errors;

pub type PgConnection = PooledConnection<PostgresConnectionManager>;
pub mod models;

pub struct DbExecutor(pub Pool<PostgresConnectionManager>);

impl Actor for DbExecutor {
    type Context = SyncContext<Self>;
}

pub fn get_conn(pool: &Pool<PostgresConnectionManager>) -> Result<PgConnection, errors::Error> {
    pool.get().map_err(|err| {
        error!("Failed to get connection - {}", err.to_string());
        errors::Error::DBError(errors::DBError::PoolError(err))
    })
}

pub fn new_pool(database_url: String) -> Pool<PostgresConnectionManager> {
    let manager = PostgresConnectionManager::new(database_url, TlsMode::None).map_err(|err| {
        error!("Failed to create db pool - {}", err.to_string());
        errors::Error::DBError(errors::DBError::PGError(err))
    }).unwrap();

    Pool::new(manager).unwrap()
}
