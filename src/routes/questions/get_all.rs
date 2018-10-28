use actix_web::{AsyncResponder, HttpRequest, HttpResponse, FutureResponse, Result, Error};
use futures::Future;
use actix::prelude::{Handler, Message};
use db::{DbExecutor, models::Question};
use app::AppState;

#[derive(Serialize)]
pub struct AllQuestions {
    pub questions: Vec<Question>,
}

pub struct GetAllQuestions;

impl Message for GetAllQuestions {
    type Result = Result<AllQuestions, Error>;
}

impl Handler<GetAllQuestions> for DbExecutor {
    type Result = Result<AllQuestions, Error>;

    fn handle(&mut self, _: GetAllQuestions, _: &mut Self::Context) -> Self::Result {
        let connection = self.0.get_conn().unwrap();

        let results = Question::get_all(&connection)
            .expect("Error loading questions");

        Ok(AllQuestions{ questions: results })
    }
}

pub fn get_all(req: &HttpRequest<AppState>) -> FutureResponse<HttpResponse> {
    req.state().db.send(GetAllQuestions{})
        .from_err()
        .and_then(|res| {
            match res {
                Ok(all_questions) => Ok(HttpResponse::Ok().json(all_questions)),
                Err(_) => Ok(HttpResponse::InternalServerError().into()),
            }
        })
        .responder()
}