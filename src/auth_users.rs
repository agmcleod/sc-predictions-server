use actix::prelude::{Handler, Message};
use actix_web::{
    error,
    middleware::{Middleware, Started},
    AsyncResponder, Error, HttpRequest, HttpResponse, Result,
};

use futures::Future;

use app::AppState;
use db::{get_conn, DbExecutor};

struct TokenRequest {
    token: String,
}

impl Message for TokenRequest {
    type Result = Result<(), Error>;
}

impl Handler<TokenRequest> for DbExecutor {
    type Result = Result<(), Error>;

    fn handle(&mut self, request: TokenRequest, _: &mut Self::Context) -> Self::Result {
        let connection = get_conn(&self.0).unwrap();
        let rows = connection
            .query(
                "SELECT * FROM users WHERE session_id = $1",
                &[&request.token],
            )
            .unwrap();
        if rows.len() > 0 {
            Ok(())
        } else {
            Err(error::ErrorUnauthorized("Invalid token"))
        }
    }
}

pub struct AuthUsers;

impl Middleware<AppState> for AuthUsers {
    fn start(&self, req: &HttpRequest<AppState>) -> Result<Started> {
        if let Some(auth) = req.headers().get("Authorization") {
            let auth = auth.to_str();
            match auth {
                Ok(auth) => Ok(Started::Future(
                    req.state()
                        .db
                        .send(TokenRequest {
                            token: auth.to_string(),
                        })
                        .map_err(Error::from)
                        .and_then(|res| match res {
                            Ok(_) => Ok(None),
                            Err(err) => Err(err.into()),
                        })
                        .responder(),
                )),
                Err(_) => Err(error::ErrorUnauthorized("Invalid token")),
            }
        } else {
            Err(error::ErrorUnauthorized("Invalid token"))
        }
    }
}
