use actix_web::{AsyncResponder, HttpRequest, HttpResponse, FutureResponse, Result, Error};
use futures::Future;
use actix::prelude::{Handler, Message};

use db::{DbExecutor, models::Question, get_conn};
use app::AppState;

#[derive(Deserialize, Serialize)]
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
        let connection = get_conn(&self.0).unwrap();

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

#[cfg(test)]
mod tests {
    use std;

    use actix_web::{http, HttpMessage};
    use chrono::{Utc, TimeZone};
    use serde_json;

    use db::{new_pool, get_conn};

    use tests::get_server;
    use super::{AllQuestions};

    #[test]
    fn test_questions_empty() {
        let mut srv = get_server();
        let req = srv.client(http::Method::GET, "/api/questions").finish().unwrap();
        let res = srv.execute(req.send()).map_err(|err| {
            println!("{}", err);
        }).unwrap();
        assert!(res.status().is_success());

        let bytes = srv.execute(res.body()).unwrap();
        let body = std::str::from_utf8(&bytes).unwrap();
        let response: AllQuestions = serde_json::from_str(body).unwrap();

        println!("{:?}", response.questions);

        assert_eq!(response.questions.len(), 0);
    }

    #[test]
    fn test_questions_populated() {
        let mut srv = get_server();
        use std::env;

        let database_url = env::var("DATABASE_URL")
            .expect("DATABASE_URL must be set");

        let pool = new_pool(database_url);
        let conn = get_conn(&pool).unwrap();

        conn.execute(
            "INSERT INTO questions (body, created_at, updated_at) VALUES ('This is the question', $1, $2)",
            &[
                &Utc.ymd(2017, 12, 10).and_hms(0, 0, 0),
                &Utc.ymd(2017, 12, 10).and_hms(0, 0, 0),
            ],
        ).unwrap();

        let req = srv.client(http::Method::GET, "/api/questions").finish().unwrap();
        let res = srv.execute(req.send()).map_err(|err| {
            println!("{}", err);
        }).unwrap();
        assert!(res.status().is_success());

        let bytes = srv.execute(res.body()).unwrap();
        let body = std::str::from_utf8(&bytes).unwrap();
        let response: AllQuestions = serde_json::from_str(body).unwrap();

        assert_eq!(response.questions.len(), 1);
        assert_eq!(response.questions[0].body, "This is the question");

        conn.execute(
            "DELETE FROM questions", &[]
        ).unwrap();
    }
}
