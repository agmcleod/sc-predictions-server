#[cfg(test)]
pub mod tests {

    use actix_identity::Identity;
    use actix_web::{test, App, FromRequest};
    use serde::{de::DeserializeOwned, Serialize};
    use serde_json;

    use crate::auth::get_identity_service;
    use crate::db;
    use crate::routes::routes;

    /// Helper for HTTP GET integration tests
    pub async fn test_get<R>(route: &str) -> (u16, R)
    where
        R: DeserializeOwned,
    {
        let mut app = test::init_service(
            App::new()
                .wrap(get_identity_service())
                .data(db::new_pool())
                .configure(routes),
        )
        .await;

        let res =
            test::call_service(&mut app, test::TestRequest::get().uri(route).to_request()).await;

        let status = res.status().as_u16();
        let body = test::read_body(res).await;
        let json_body = serde_json::from_slice(&body).unwrap_or_else(|_| {
            panic!(
                "read_response_json failed during deserialization. response: {}",
                String::from_utf8(body.to_vec())
                    .unwrap_or_else(|_| "Could not convert Bytes -> String".to_string())
            )
        });

        (status, json_body)
    }

    /// Helper for HTTP POST integration tests
    pub async fn test_post<T: Serialize, R>(route: &str, params: T) -> (u16, R)
    where
        R: DeserializeOwned,
    {
        let mut app = test::init_service(
            App::new()
                .wrap(get_identity_service())
                .data(db::new_pool())
                .configure(routes),
        )
        .await;
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
        let json_body = serde_json::from_slice(&body).unwrap_or_else(|_| {
            panic!(
                "read_response_json failed during deserialization. response: {}",
                String::from_utf8(body.to_vec())
                    .unwrap_or_else(|_| "Could not convert Bytes -> String".to_string())
            )
        });

        (status, json_body)
    }

    pub async fn get_identity() -> Identity {
        let (request, mut payload) =
            test::TestRequest::with_header("content-type", "application/json").to_http_parts();
        Option::<Identity>::from_request(&request, &mut payload)
            .await
            .unwrap()
            .unwrap()
    }
}
