use actix::prelude::Addr;
use db::DbExecutor;
use actix_web::{http, App, Responder, HttpRequest, middleware::Logger};
use routes::questions;

pub struct AppState {
    pub db: Addr<DbExecutor>,
}

pub fn index(_: &HttpRequest<AppState>) -> impl Responder {
    "Welcome"
}

pub fn create_app(db: Addr<DbExecutor>) ->  App<AppState> {
    App::with_state(AppState{ db })
        .middleware(Logger::default())
        .middleware(Logger::new("%a %{User-Agent}i"))
        .resource("/api/questions", |r| r.method(http::Method::GET).f(questions::get_all))
        .resource("/", |r| r.method(http::Method::GET).f(index))
}
