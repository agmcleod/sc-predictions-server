#[cfg(test)]
pub mod tests {

    use actix_web::{dev::ServiceResponse, test, App};
    use serde::{de::DeserializeOwned, Serialize};
    use serde_json;

    use crate::db;
    use crate::routes::routes;

    /// Helper for HTTP GET integration tests
    pub async fn test_get(route: &str) -> ServiceResponse {
        let mut app = test::init_service(App::new().data(db::new_pool()).configure(routes)).await;

        test::call_service(&mut app, test::TestRequest::get().uri(route).to_request()).await
    }

    /// Helper for HTTP POST integration tests
    pub async fn test_post<T: Serialize, R>(route: &str, params: T) -> (u16, R)
    where
        R: DeserializeOwned,
    {
        let mut app = test::init_service(App::new().data(db::new_pool()).configure(routes)).await;
        let res = test::call_service(
            &mut app,
            test::TestRequest::post()
                .set_json(&params)
                .uri(route)
                .to_request(),
        )
        .await;

        let status = res.status().as_u16();
        let body = test::read_body(res).await;
        let json_body = serde_json::from_slice(&body)
            .unwrap_or_else(|_| panic!("read_response_json failed during deserialization"));

        (status, json_body)
    }
}
