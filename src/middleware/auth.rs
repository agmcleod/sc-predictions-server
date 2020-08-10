use std::pin::Pin;
use std::task::{Context, Poll};

use actix_identity::RequestIdentity;
use actix_service::{Service, Transform};
use actix_web::{
    dev::{ServiceRequest, ServiceResponse},
    Error, HttpResponse,
};
use futures::{
    future::{ok, Ready},
    Future,
};

use crate::auth::{decode_jwt, PrivateClaim};
use crate::errors;

pub struct Auth;

impl<S, B> Transform<S> for Auth
where
    S: Service<Request = ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
{
    type Request = ServiceRequest;
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = AuthMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(AuthMiddleware { service })
    }
}

pub struct AuthMiddleware<S> {
    service: S,
}

impl<S, B> Service for AuthMiddleware<S>
where
    S: Service<Request = ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
{
    type Request = ServiceRequest;
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;

    fn poll_ready(&mut self, cx: &mut Context) -> Poll<Result<(), Self::Error>> {
        self.service.poll_ready(cx)
    }

    fn call(&mut self, req: ServiceRequest) -> Self::Future {
        let identity = RequestIdentity::get_identity(&req).unwrap_or("".into());
        let private_claim: Result<PrivateClaim, errors::Error> = decode_jwt(&identity);

        // decode uses default validation to ensure not expired, changed, etc.
        if private_claim.is_ok() {
            let fut = self.service.call(req);
            Box::pin(async move {
                let res = fut.await?;
                Ok(res)
            })
        } else {
            Box::pin(async move {
                Ok(req.into_response(
                    HttpResponse::Unauthorized()
                        .json::<errors::ErrorResponse>("Unauthorized".into())
                        .into_body(),
                ))
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::{Duration, Utc};

    use crate::auth::PrivateClaim;
    use crate::errors::ErrorResponse;
    use crate::tests::helpers::tests::{get_auth_token, test_get};

    #[actix_rt::test]
    async fn test_expired_token_unauthorized() {
        let mut claim = PrivateClaim::new(1, "".to_string(), 1);
        claim.set_exp((Utc::now() - Duration::minutes(1)).timestamp());
        let cookie = get_auth_token(claim);
        let res = test_get(&format!("/api/games/{}/players", 1), Some(cookie)).await;
        assert_eq!(res.0, 401);

        let body: ErrorResponse = res.1;
        assert_eq!(body.errors.get(0).unwrap(), "Unauthorized");
    }
}
