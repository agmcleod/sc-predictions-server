use actix_web::{HttpRequest, Result, Json};
use db::{establish_connection, models::Question};
use schema::questions::dsl::*;
use diesel::prelude::*;

#[derive(Serialize)]
pub struct AllQuestions {
    pub questions: Vec<Question>,
}

pub fn get_all(_: &HttpRequest) -> Result<Json<AllQuestions>> {
    let connection = establish_connection();

    let results = questions
        .load::<Question>(&connection)
        .expect("Error loading questions");

    Ok(Json(AllQuestions{ questions: results }))
}