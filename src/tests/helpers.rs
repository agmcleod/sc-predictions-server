#[cfg(test)]
pub mod tests {
    use std::time::SystemTime;

    use actix_http::Request;
    use actix_identity::Identity;
    use actix_service::Service;
    use actix_web::{body::Body, dev::ServiceResponse, error::Error, test, App, FromRequest};
    use serde::{de::DeserializeOwned, Deserialize, Serialize};
    use serde_json;

    use crate::auth::{create_jwt, get_identity_service, PrivateClaim};
    use crate::db;
    use crate::routes::routes;

    #[derive(Deserialize, Serialize, Debug)]
    struct CookieValue {
        identity: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        login_timestamp: Option<SystemTime>,
        #[serde(skip_serializing_if = "Option::is_none")]
        visit_timestamp: Option<SystemTime>,
    }

    pub async fn get_service(
    ) -> impl Service<Request = Request, Response = ServiceResponse<Body>, Error = Error> {
        test::init_service(
            App::new()
                .wrap(get_identity_service())
                .data(db::new_pool())
                .configure(routes),
        )
        .await
    }

    /// Helper for HTTP GET integration tests
    pub async fn test_get<R>(route: &str, token: Option<String>) -> (u16, R)
    where
        R: DeserializeOwned,
    {
        let mut app = get_service().await;
        let mut req = test::TestRequest::get().uri(route);
        if let Some(token) = token {
            req = req.header("Authorization", token);
        }

        let res = test::call_service(&mut app, req.to_request()).await;

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
    pub async fn test_post<T: Serialize, R>(
        route: &str,
        params: T,
        token: Option<String>,
    ) -> (u16, R)
    where
        R: DeserializeOwned,
    {
        let mut app = get_service().await;

        let mut req = test::TestRequest::post().set_json(&params).uri(route);
        if let Some(token) = token {
            req = req.header("Authorization", token);
        }

        let res = test::call_service(&mut app, req.to_request()).await;

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

    pub fn get_auth_token(private_claim: PrivateClaim) -> String {
        create_jwt(private_claim).unwrap()
    }
}
