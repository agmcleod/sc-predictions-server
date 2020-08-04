#[macro_use]
extern crate diesel;
#[macro_use]
extern crate log;
#[macro_use]
extern crate validator_derive;

use std::env;

use actix_cors::Cors;
use actix_rt;
use actix_web::{middleware::Logger, App, HttpServer};
use dotenv::dotenv;
use env_logger;

mod auth;
mod db;
mod errors;
mod middleware;
mod routes;
mod schema;
mod tests;
mod utils;
mod validate;

use routes::routes;

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    env_logger::init();

    let pool = db::new_pool();

    HttpServer::new(move || {
        let cors = Cors::new()
            .allowed_origin(&env::var("CLIENT_HOST").unwrap())
            .max_age(3600)
            .finish();

        App::new()
            .wrap(cors)
            .wrap(Logger::default())
            .wrap(Logger::new("%a %{User-Agent}i"))
            .wrap(auth::get_identity_service())
            .data(pool.clone())
            .configure(routes)
    })
    .bind("0.0.0.0:8080")?
    .run()
    .await
}
