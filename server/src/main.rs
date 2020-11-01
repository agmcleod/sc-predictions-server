#[cfg(test)]
#[macro_use]
extern crate diesel;
#[macro_use]
extern crate validator_derive;

use std::env;

use actix_cors::Cors;
use actix_rt;
use actix_web::{middleware::Logger, web, App, HttpResponse, HttpServer, http};
use dotenv::dotenv;
use env_logger;

mod middleware;
mod routes;
mod tests;
mod validate;

use crate::routes::routes;
use errors::ErrorResponse;

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    env_logger::init();

    let pool = db::new_pool();

    HttpServer::new(move || {
        let cors = Cors::default()
            .allowed_origin(&env::var("CLIENT_HOST").unwrap())
            .allow_any_method()
            .allowed_headers(vec![http::header::AUTHORIZATION, http::header::ACCEPT, http::header::CONTENT_TYPE])
            // .allow_any_header()
            .max_age(3600);

        App::new()
            .wrap(cors)
            .wrap(Logger::default())
            .wrap(Logger::new("%a %{User-Agent}i"))
            .wrap(auth::get_identity_service())
            .data(pool.clone())
            .configure(routes)
            .default_service(
                web::route()
                    .to(|| HttpResponse::NotFound().json::<ErrorResponse>("Not Found".into())),
            )
    })
    .bind("0.0.0.0:8080")?
    .run()
    .await
}
