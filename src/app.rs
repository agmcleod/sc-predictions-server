use std::env;

use actix::prelude::Addr;
use db::DbExecutor;
use actix_web::{http, App, middleware::{cors, Logger}};
use routes::questions;

pub struct AppState {
    pub db: Addr<DbExecutor>,
}

pub fn create_app(db: Addr<DbExecutor>) ->  App<AppState> {
    let cors = cors::Cors::build()
        .allowed_origin(&env::var("CLIENT_HOST").unwrap())
        .max_age(3600)
        .finish();

    App::with_state(AppState{ db })
        .middleware(Logger::default())
        .middleware(Logger::new("%a %{User-Agent}i"))
        .middleware(cors)
        .resource("/api/questions", |r| r.method(http::Method::GET).f(questions::get_all))
}
