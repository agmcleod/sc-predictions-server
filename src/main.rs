extern crate actix;
extern crate actix_web;
extern crate chrono;
extern crate futures;
extern crate env_logger;
#[macro_use] extern crate log;
extern crate serde;
extern crate serde_json;
#[macro_use] extern crate serde_derive;
extern crate postgres;
extern crate postgres_mapper;
#[macro_use] extern crate postgres_mapper_derive;
extern crate dotenv;
extern crate r2d2;
extern crate r2d2_postgres;

use std::env;

use actix::{Addr, SyncArbiter, System};
use actix_web::{server};
use dotenv::dotenv;
use r2d2_postgres::{PostgresConnectionManager, TlsMode};

mod app;
mod errors;
mod routes;
mod db;

use app::create_app;
use db::{DbExecutor};

fn new_pool(database_url: String) -> r2d2::Pool<PostgresConnectionManager> {
    let manager = PostgresConnectionManager::new(database_url, TlsMode::None).map_err(|err| {
        error!("Failed to create db pool - {}", err.to_string());
        errors::Error::DBError(errors::DBError::PGError(err))
    }).unwrap();

    r2d2::Pool::new(manager).unwrap()
}

fn main() {
    dotenv().ok();
    env::set_var("RUST_LOG", "actix_web=info");
    env_logger::init();

    let database_url = env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");

    let sys = System::new("sc-predictions");

    let pool = new_pool(database_url);

    // start arbiter with 4 threads
    // usage of move for the closure lets it take ownership of pool, before cloning it
    let address: Addr<DbExecutor> = SyncArbiter::start(4, move || DbExecutor(pool.clone()));

    server::new(move || create_app(address.clone()))
        .bind("0.0.0.0:8080")
        .expect("Can not bind to :8080")
        .start();

    sys.run();
}

#[cfg(test)]
mod tests {
    use actix_web::{http, HttpMessage, test::{TestServer}};
    use super::*;

    fn test_welcome(srv: &mut TestServer) {
        let req = srv.get().finish().unwrap();
        let res = srv.execute(req.send()).unwrap();
        assert!(res.status().is_success());

        let bytes = srv.execute(res.body()).unwrap();
        let body = std::str::from_utf8(&bytes).unwrap();
        assert_eq!(body, "Welcome");
    }

    fn test_questions(srv: &mut TestServer) {
        let req = srv.client(http::Method::GET, "/api/questions").finish().unwrap();
        let res = srv.execute(req.send()).map_err(|err| {
            println!("{}", err);
        }).unwrap();
        assert!(res.status().is_success());

        let bytes = srv.execute(res.body()).unwrap();
        let body = std::str::from_utf8(&bytes).unwrap();
        let response: routes::questions::AllQuestions = serde_json::from_str(body).unwrap();

        assert!(response.questions.len() == 0);
    }

    #[test]
    fn integration() {
        dotenv().ok();
        let mut srv = TestServer::build_with_state(|| {
            let database_url = env::var("DATABASE_URL")
                .expect("DATABASE_URL must be set");
            let pool = new_pool(database_url);
            let addr = SyncArbiter::start(4, move || DbExecutor(pool.clone()));
            app::AppState{db: addr}
        })
        // register server handlers and start test server
        .start(|app| {
            app
                .resource("/api/questions", |r| r.method(http::Method::GET).f(routes::questions::get_all))
                .resource("/", |r| r.method(http::Method::GET).f(app::index));
        });

        test_welcome(&mut srv);
        test_questions(&mut srv);
    }
}
