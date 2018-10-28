use postgres::transaction::Transaction;
use r2d2::{Pool as PgPool, PooledConnection};
use r2d2_postgres::{PostgresConnectionManager, TlsMode};

use errors;

pub type PgConnection = PooledConnection<PostgresConnectionManager>;

#[derive(Clone)]
pub struct Pool {
    pub inner: PgPool<PostgresConnectionManager>,
}

impl Pool {
    pub fn from_url(db_url: &str) -> Result<Pool, errors::Error> {
        PostgresConnectionManager::new(db_url, TlsMode::None).map_err(|err| {
            error!("Failed to create db pool - {}", err.to_string());
            errors::Error::DBError(errors::DBError::PGError(err))
        })
        .and_then(|manager| {
            PgPool::new(manager).map_err(|err| {
                error!("Failed to create db pool - {}", err.to_string());
                errors::Error::DBError(errors::DBError::PoolError(err))
            })
            .and_then(|pool| Ok(Pool{ inner: pool }))
        })
    }

    pub fn get_conn(&self) -> Result<PgConnection, errors::Error> {
        self.inner.get().map_err(|err| {
            error!("Failed to create db connection - {}", err.to_string());
            errors::Error::DBError(errors::DBError::PoolError(err))
        })
    }

    pub fn get_transaction<'t>(&self, conn: &'t PgConnection) -> Result<Transaction<'t>, errors::Error> {
        conn.transaction().map_err(|err| {
            error!("Failed to create transaction - {}", err.to_string());
            errors::Error::DBError(errors::DBError::PGError(err))
        })
    }


}