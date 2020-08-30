use diesel::{self, ExpressionMethods, RunQueryDsl};
use dotenv::dotenv;

use db::{get_conn, new_pool, schema::questions};

fn main() {
    dotenv().ok();

    let pool = new_pool();
    let conn = get_conn(&pool).unwrap();

    for body in &[
        "First to expand",
        "First to scout a building",
        "First to max supply, or highest supply",
        "Who to win",
    ] {
        diesel::insert_into(questions::table)
            .values(questions::dsl::body.eq(body))
            .execute(&conn)
            .unwrap();
    }
}
