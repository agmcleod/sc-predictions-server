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
        let is_logged_in = private_claim.is_ok();

        if is_logged_in {
            let fut = self.service.call(req);
            Box::pin(async move {
                let res = fut.await?;
                Ok(res)
            })
        } else {
            // Box::pin(async move {
            //     Ok(req.into_response(HttpResponse::Unauthorized().finish().into_body()))
            // })

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
