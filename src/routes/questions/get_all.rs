use actix_web::{web, Error, HttpResponse, Result};

use crate::db::{get_conn, models::Question, PgPool};

pub async fn get_all(pool: web::Data<PgPool>) -> Result<HttpResponse, Error> {
    let connection = get_conn(&pool).unwrap();

    let questions = Question::get_all(&connection)?;

    Ok(HttpResponse::Ok().json(questions))
}

#[cfg(test)]
mod tests {
    use std;

    use actix_web::http;
    use chrono::{TimeZone, Utc};
    use serde_json;

    use crate::app_tests::{get_server, POOL};
    use crate::db::{get_conn, models::Question};

    #[test]
    fn test_questions_empty() {
        let mut srv = get_server();
        let req = srv
            .client(http::Method::GET, "/api/questions")
            .finish()
            .unwrap();
        let res = srv
            .execute(req.send())
            .map_err(|err| {
                println!("{}", err);
            })
            .unwrap();
        assert!(res.status().is_success());

        let bytes = srv.execute(res.body()).unwrap();
        let body = std::str::from_utf8(&bytes).unwrap();
        let response: Vec<Question> = serde_json::from_str(body).unwrap();

        assert_eq!(response.questions.len(), 0);
    }

    #[test]
    fn test_questions_populated() {
        let conn = get_conn(&POOL).unwrap();

        conn.execute(
            "INSERT INTO questions (body, created_at, updated_at) VALUES ('This is the question', $1, $2)",
            &[
                &Utc.ymd(2017, 12, 10).and_hms(0, 0, 0),
                &Utc.ymd(2017, 12, 10).and_hms(0, 0, 0),
            ],
        ).unwrap();

        let mut srv = get_server();

        let req = srv
            .client(http::Method::GET, "/api/questions")
            .finish()
            .unwrap();
        let res = srv
            .execute(req.send())
            .map_err(|err| {
                println!("{}", err);
            })
            .unwrap();
        assert!(res.status().is_success());

        let bytes = srv.execute(res.body()).unwrap();
        let body = std::str::from_utf8(&bytes).unwrap();
        let response: Vec<Question> = serde_json::from_str(body).unwrap();

        assert_eq!(response.questions.len(), 1);
        assert_eq!(response.questions[0].body, "This is the question");

        conn.execute("DELETE FROM questions", &[]).unwrap();
    }
}
