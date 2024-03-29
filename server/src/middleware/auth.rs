use std::pin::Pin;
use std::task::{Context, Poll};

use actix_identity::RequestIdentity;
use actix_service::{Service, Transform};
use actix_web::{
    body::BoxBody,
    dev::{ServiceRequest, ServiceResponse},
    Error, HttpResponse,
};
use futures::{
    future::{ok, Ready},
    Future,
};

use auth::{decode_jwt, PrivateClaim};
use errors;

pub struct Auth;

impl<S> Transform<S, ServiceRequest> for Auth
where
    S: Service<ServiceRequest, Response = ServiceResponse<BoxBody>, Error = Error>,
    S::Future: 'static,
{
    type Response = ServiceResponse<BoxBody>;
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

impl<S> Service<ServiceRequest> for AuthMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<BoxBody>, Error = Error>,
    S::Future: 'static,
{
    type Response = ServiceResponse<BoxBody>;
    type Error = Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;

    fn poll_ready(&self, cx: &mut Context) -> Poll<Result<(), Self::Error>> {
        self.service.poll_ready(cx)
    }

    fn call(&self, req: ServiceRequest) -> Self::Future {
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
                let error: errors::ErrorResponse = "Unauthorized".into();
                Ok(req.into_response(HttpResponse::Unauthorized().json(error)))
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::{Duration, Utc};

    use auth::{PrivateClaim, Role};
    use errors::ErrorResponse;

    use crate::tests::helpers::tests::{get_auth_token, test_get};

    #[actix_rt::test]
    async fn test_expired_token_unauthorized() {
        let mut claim = PrivateClaim::new(1, "".to_string(), 1, Role::Owner);
        claim.set_exp((Utc::now() - Duration::minutes(1)).timestamp());
        let cookie = get_auth_token(claim);
        let res = test_get(&format!("/api/games/{}/players", 1), Some(cookie)).await;
        assert_eq!(res.0, 401);

        let body: ErrorResponse = res.1;
        assert_eq!(body.errors.get(0).unwrap(), "Unauthorized");
    }
}
