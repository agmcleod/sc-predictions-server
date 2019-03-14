use actix_web::{error, http, HttpResponse};
use postgres::error::Error as PGError;
use r2d2::Error as PoolError;
use serde_json;
use validator::ValidationErrors;

#[derive(Debug)]
pub enum DBError {
    MapError(String),
    NoRecord,
    PGError(PGError),
    PoolError(PoolError),
}

#[derive(Fail, Debug)]
pub enum Error {
    #[fail(display = "db error")]
    DBError(DBError),
    #[fail(display = "validation error")]
    ValidationError(ValidationErrors),
}

impl error::ResponseError for Error {
    fn error_response(&self) -> HttpResponse {
        match *self {
            Error::DBError(ref db_err) => match db_err {
                DBError::NoRecord => HttpResponse::new(http::StatusCode::NOT_FOUND),
                _ => HttpResponse::new(http::StatusCode::INTERNAL_SERVER_ERROR),
            },
            Error::ValidationError(ref validation_errors) => {
                match serde_json::to_string(&validation_errors.clone().errors()) {
                    Ok(json) => HttpResponse::from(json.to_string()),
                    Err(err) => HttpResponse::from(err.to_string()),
                }
            }
        }
    }
}

impl From<ValidationErrors> for Error {
    fn from(errors: ValidationErrors) -> Self {
        Error::ValidationError(errors)
    }
}
