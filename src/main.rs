#[macro_use]
extern crate diesel;
#[macro_use]
extern crate failure;
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

mod db;
mod errors;
mod routes;
mod schema;
mod tests;
mod utils;

use routes::routes;

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    env::set_var("RUST_LOG", "actix_web=info");
    env_logger::init();

    let pool = db::new_pool();

    HttpServer::new(move || {
        let cors = Cors::new()
            .allowed_origin(&env::var("CLIENT_HOST").unwrap())
            .max_age(3600)
            .finish();

        App::new()
            .data(pool.clone())
            .wrap(cors)
            .wrap(Logger::default())
            .wrap(Logger::new("%a %{User-Agent}i"))
            .configure(routes)
    })
    .bind(&"127.0.0.1:8080")?
    .run()
    .await
}
