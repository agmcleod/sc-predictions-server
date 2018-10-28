use std::fmt;

use postgres::error::Error as PGError;
use r2d2::Error as PoolError;

#[derive(Debug)]
pub enum DBError {
    NoRecord,
    PGError(PGError),
    PoolError(PoolError),
}

#[derive(Debug)]
pub enum Error {
    DBError(DBError),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &Error::DBError(ref err) => write!(f, "DB Error: {:?}", err),
        }
    }
}
