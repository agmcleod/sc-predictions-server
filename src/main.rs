extern crate actix_web;
extern crate chrono;
extern crate env_logger;
extern crate serde;
#[macro_use] extern crate serde_derive;
#[macro_use] extern crate diesel;
extern crate dotenv;
use actix_web::{http, server, App, Path, Responder, Result, HttpRequest, middleware::Logger};

mod routes;
mod db;
mod schema;

use routes::questions;

fn index(_: &HttpRequest) -> impl Responder {
    "Welcome"
}

fn main() {
    std::env::set_var("RUST_LOG", "actix_web=info");
    env_logger::init();

    server::new(|| {
        App::new()
            .middleware(Logger::default())
            .middleware(Logger::new("%a %{User-Agent}i"))
            .resource("/questions", |r| r.method(http::Method::GET).f(questions::get_all))
            .resource("/", |r| r.method(http::Method::GET).f(index))
    })
    .bind("0.0.0.0:8080").expect("Can not bind to :8080")
    .run();
}
