use actix_web::{
    web::{block, Data, Json},
    Result,
};

use db::{get_conn, models::Question, PgPool};
use errors::Error;

pub async fn get_all(pool: Data<PgPool>) -> Result<Json<Vec<Question>>, Error> {
    let connection = get_conn(&pool).unwrap();

    let questions = block(move || Question::get_all(&connection)).await?;

    Ok(Json(questions))
}

#[cfg(test)]
mod tests {
    use diesel::{self, RunQueryDsl};

    use crate::tests::helpers::tests::test_get;
    use db::{get_conn, models::Question, new_pool, schema::questions};

    #[derive(Insertable)]
    #[table_name = "questions"]
    struct NewQuestion {
        body: String,
    }

    #[actix_rt::test]
    async fn test_questions_empty() {
        let res: (u16, Vec<Question>) = test_get("/api/questions", None).await;
        assert_eq!(res.0, 200);

        assert_eq!(res.1.len(), 0);
    }

    #[actix_rt::test]
    async fn test_questions_populated() {
        let pool = new_pool();
        let conn = get_conn(&pool).unwrap();

        diesel::insert_into(questions::table)
            .values(NewQuestion {
                body: "This is the question".to_string(),
            })
            .execute(&conn)
            .unwrap();

        let res = test_get("/api/questions", None).await;
        assert_eq!(res.0, 200);

        let body: Vec<Question> = res.1;

        assert_eq!(body.len(), 1);
        assert_eq!(body[0].body, "This is the question");

        diesel::delete(questions::table).execute(&conn).unwrap();
    }
}
