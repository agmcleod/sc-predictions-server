extern crate actix_web;
extern crate env_logger;
use actix_web::{http, server, App, Path, Responder, middleware::Logger};

fn index(info: Path<(u32, String)>) -> impl Responder {
    format!("Hello {}! id: {}", info.1, info.0)
}

fn main() {
    std::env::set_var("RUST_LOG", "actix_web=info");
    env_logger::init();

    server::new(|| {
        App::new()
            .middleware(Logger::default())
            .middleware(Logger::new("%a %{User-Agent}i"))
            .route("/{id}/{name}", http::Method::GET, index)
    })
    .bind("0.0.0.0:8080").expect("Can not bind to :8080")
    .run();
}
