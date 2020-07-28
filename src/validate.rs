use actix_web::web::Json;
use validator::{Validate, ValidationErrors};

use crate::errors::Error;

fn collect_errors(errors: ValidationErrors) -> Vec<String> {
    errors
        .field_errors()
        .into_iter()
        .map(|err| {
            let default_error = format!("{} is required", err.0);
            err.1[0]
                .message
                .as_ref()
                .unwrap_or(&std::borrow::Cow::Owned(default_error))
                .to_string()
        })
        .collect()
}

pub fn validate<T>(params: &Json<T>) -> Result<(), Error>
where
    T: Validate,
{
    match params.validate() {
        Ok(_) => Ok(()),
        Err(err) => Err(Error::ValidationError(collect_errors(err))),
    }
}
