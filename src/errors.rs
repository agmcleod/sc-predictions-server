use actix_web::{error, http, HttpResponse};
use r2d2::Error as PoolError;
use serde::Serialize;
use serde_json;
use validator::ValidationErrors;
#[derive(Fail, Debug)]
pub enum Error {
    #[fail(display = "validation error")]
    ValidationError(ValidationErrors),
    #[fail(display = "json payload error")]
    JsonPayload(error::JsonPayloadError),
}

#[derive(Serialize)]
pub struct JsonErrorBody {
    error: String,
}

impl JsonErrorBody {
    fn new(msg: String) -> Self {
        JsonErrorBody { error: msg }
    }
}

impl error::ResponseError for Error {
    fn error_response(&self) -> HttpResponse {
        match *self {
            Error::ValidationError(ref validation_errors) => {
                match serde_json::to_string(&validation_errors.clone().errors()) {
                    Ok(json) => {
                        HttpResponse::build(http::StatusCode::UNPROCESSABLE_ENTITY).json(json)
                    }
                    Err(err) => HttpResponse::from(err.to_string()),
                }
            }
            Error::JsonPayload(ref json_payload_err) => {
                HttpResponse::build(http::StatusCode::BAD_REQUEST)
                    .json(JsonErrorBody::new(json_payload_err.to_string()))
            }
        }
    }
}

impl From<ValidationErrors> for Error {
    fn from(errors: ValidationErrors) -> Self {
        Error::ValidationError(errors)
    }
}
