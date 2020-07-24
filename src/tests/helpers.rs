#[cfg(test)]
pub mod tests {
    use std::env;

    use actix_web::{dev::ServiceResponse, test, App};
    use dotenv::dotenv;
    use lazy_static::lazy_static;
    use r2d2::Pool;
    use r2d2_postgres::PostgresConnectionManager;
    use serde::Serialize;

    use crate::db;
    use crate::routes::routes;

    lazy_static! {
        pub static ref POOL: Pool<PostgresConnectionManager> = {
            dotenv().ok();
            let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
            db::new_pool(database_url)
        };
    }

    /// Helper for HTTP GET integration tests
    pub async fn test_get(route: &str) -> ServiceResponse {
        let mut app = test::init_service(App::new().data(POOL.clone()).configure(routes)).await;

        test::call_service(&mut app, test::TestRequest::get().uri(route).to_request()).await
    }

    /// Helper for HTTP POST integration tests
    pub async fn test_post<T: Serialize>(route: &str, params: T) -> ServiceResponse {
        let mut app = test::init_service(App::new().data(POOL.clone()).configure(routes)).await;
        test::call_service(
            &mut app,
            test::TestRequest::post()
                .set_json(&params)
                .uri(route)
                .to_request(),
        )
        .await
    }
}
