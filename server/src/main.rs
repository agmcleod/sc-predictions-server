#[cfg(test)]
#[macro_use]
extern crate diesel;
#[macro_use]
extern crate log;
#[macro_use]
extern crate validator_derive;

use std::env;

use actix::Actor;
use actix_cors::Cors;
use actix_rt;
use actix_web::{http, middleware::Logger, web, App, HttpResponse, HttpServer};
use dotenv::dotenv;
use env_logger;

mod handlers;
mod middleware;
mod routes;
mod tests;
mod validate;
mod websocket;

use crate::routes::routes;
use db;

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("debug"));

    let pool = db::new_pool();

    let server = websocket::Server::new(pool.clone()).start();

    HttpServer::new(move || {
        let cors = Cors::default()
            .allowed_origin(&env::var("CLIENT_HOST").unwrap())
            .allow_any_method()
            .allowed_headers(vec![
                http::header::AUTHORIZATION,
                http::header::ACCEPT,
                http::header::CONTENT_TYPE,
            ])
            // .allow_any_header()
            .max_age(3600);

        App::new()
            .wrap(cors)
            .wrap(Logger::default())
            .wrap(Logger::new("%a %{User-Agent}i"))
            .wrap(auth::get_identity_service())
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(server.clone()))
            .configure(routes)
            .default_service(web::to(|| HttpResponse::NotFound()))
    })
    .bind("0.0.0.0:8080")?
    .run()
    .await
}
