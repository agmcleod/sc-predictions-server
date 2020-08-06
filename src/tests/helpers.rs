#[cfg(test)]
pub mod tests {
    use std::env;
    use std::time::SystemTime;

    use actix_http::Request;
    use actix_identity::Identity;
    use actix_service::Service;
    use actix_web::{
        body::Body,
        cookie::{Cookie, CookieJar, Key},
        dev::ServiceResponse,
        error::Error,
        test, App, FromRequest,
    };
    use serde::{de::DeserializeOwned, Deserialize, Serialize};
    use serde_json;

    use crate::auth::{create_jwt, get_identity_service, PrivateClaim, SESSION_NAME};
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
    pub async fn test_get<R>(route: &str, cookie: Option<Cookie<'static>>) -> (u16, R)
    where
        R: DeserializeOwned,
    {
        let mut app = get_service().await;
        let mut req = test::TestRequest::get().uri(route);
        if let Some(cookie) = cookie {
            req = req.cookie(cookie);
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
        cookie: Option<Cookie<'static>>,
    ) -> (u16, R)
    where
        R: DeserializeOwned,
    {
        let mut app = get_service().await;

        let mut req = test::TestRequest::post().set_json(&params).uri(route);
        if let Some(cookie) = cookie {
            req = req.cookie(cookie);
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

    pub async fn get_identity() -> Identity {
        let (request, mut payload) =
            test::TestRequest::with_header("content-type", "application/json").to_http_parts();
        Option::<Identity>::from_request(&request, &mut payload)
            .await
            .unwrap()
            .unwrap()
    }

    pub fn get_login_cookie(
        private_claim: PrivateClaim,
        login_timestamp: Option<SystemTime>,
    ) -> Cookie<'static> {
        let login_time = login_timestamp.unwrap_or_else(|| SystemTime::now());
        let identity = create_jwt(private_claim).unwrap();

        let mut jar = CookieJar::new();
        let key: Vec<u8> = env::var("SESSION_KEY")
            .unwrap()
            .into_bytes()
            .iter()
            .chain([1, 0, 0, 0].iter())
            .copied()
            .collect();

        jar.private(&Key::from_master(&key)).add(Cookie::new(
            SESSION_NAME,
            serde_json::to_string(&CookieValue {
                identity,
                login_timestamp: Some(login_time),
                visit_timestamp: None,
            })
            .unwrap(),
        ));

        jar.get(SESSION_NAME).unwrap().clone()
    }
}
