#[macro_use]
extern crate log;

use actix_web::{
    error::{BlockingError, ResponseError},
    Error as ActixError, HttpResponse,
};
use derive_more::Display;
use diesel::result::{DatabaseErrorKind, Error as DBError};
use r2d2::Error as PoolError;
use serde::{Deserialize, Serialize};

#[derive(Debug, Display, PartialEq)]
#[allow(dead_code)]
pub enum Error {
    BadRequest(String),
    CannotDecodeJwtToken(String),
    CannotEncodeJwtToken(String),
    InternalServerError(String),
    Unauthorized,
    Forbidden,
    NotFound(String),
    PoolError(String),
    #[display(fmt = "")]
    ValidationError(Vec<String>),
    UnprocessableEntity(String),
    BlockingError(String),
}

// User-friendly error messages
#[derive(Debug, Deserialize, Serialize)]
pub struct ErrorResponse {
    pub errors: Vec<String>,
}

impl ResponseError for Error {
    fn error_response(&self) -> HttpResponse {
        match self {
            Error::ValidationError(ref validation_errors) => {
                HttpResponse::UnprocessableEntity().json(validation_errors.to_vec())
            }
            Error::BadRequest(message) => {
                let error: ErrorResponse = message.into();
                HttpResponse::BadRequest().json(error)
            }
            Error::NotFound(message) => {
                let error: ErrorResponse = message.into();
                HttpResponse::NotFound().json(error)
            }
            Error::UnprocessableEntity(message) => {
                let error: ErrorResponse = message.into();
                HttpResponse::UnprocessableEntity().json(error)
            }
            Error::Forbidden => {
                let error: ErrorResponse = "Forbidden".into();
                HttpResponse::Forbidden().json(error)
            }
            _ => {
                error!("Internal server error: {:?}", self);
                let error: ErrorResponse = "Internal Server Error".into();
                HttpResponse::InternalServerError().json(error)
            }
        }
    }
}

impl From<&str> for ErrorResponse {
    fn from(error: &str) -> Self {
        ErrorResponse {
            errors: vec![error.into()],
        }
    }
}

impl From<&String> for ErrorResponse {
    fn from(error: &String) -> Self {
        ErrorResponse {
            errors: vec![error.into()],
        }
    }
}

impl From<Vec<String>> for ErrorResponse {
    fn from(error: Vec<String>) -> Self {
        ErrorResponse { errors: error }
    }
}

// Convert DBErrors to our Error type
impl From<DBError> for Error {
    fn from(error: DBError) -> Error {
        // Right now we just care about UniqueViolation from diesel
        // But this would be helpful to easily map errors as our app grows
        match error {
            DBError::DatabaseError(kind, info) => {
                if let DatabaseErrorKind::UniqueViolation = kind {
                    let message = info.details().unwrap_or_else(|| info.message()).to_string();
                    return Error::BadRequest(message);
                }
                Error::InternalServerError("Unknown database error".into())
            }
            DBError::NotFound => Error::NotFound("Record not found".into()),
            _ => Error::InternalServerError("Unknown database error".into()),
        }
    }
}

// Convert PoolError to our Error type
impl From<PoolError> for Error {
    fn from(error: PoolError) -> Error {
        Error::PoolError(error.to_string())
    }
}

impl From<BlockingError> for Error {
    fn from(error: BlockingError) -> Error {
        error!("Thread blocking error {:?}", error);
        Error::BlockingError("Thread blocking error".into())
    }
}

impl From<ActixError> for Error {
    fn from(error: ActixError) -> Error {
        Error::InternalServerError(error.to_string())
    }
}
