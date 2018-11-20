extern crate actix;
extern crate actix_web;
extern crate chrono;
extern crate futures;
extern crate env_logger;
#[macro_use] extern crate lazy_static;
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

mod app;
mod errors;
mod routes;
mod db;

use app::create_app;
use db::{DbExecutor};

fn main() {
    dotenv().ok();
    env::set_var("RUST_LOG", "actix_web=info");
    env_logger::init();

    let database_url = env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");

    let sys = System::new("sc-predictions");

    let pool = db::new_pool(database_url);

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
pub mod app_tests {
    use actix_web::{http, HttpMessage, test::{TestServer}};
    use super::*;
    use dotenv::dotenv;
    use std::env;
    use r2d2::{Pool};
    use r2d2_postgres::{PostgresConnectionManager};

    lazy_static! {
        pub static ref POOL: Pool<PostgresConnectionManager> = {
            dotenv().ok();
            let database_url = env::var("DATABASE_URL")
                .expect("DATABASE_URL must be set");
            db::new_pool(database_url)
        };
    }

    pub fn get_server() -> TestServer {
        dotenv().ok();
        TestServer::build_with_state(|| {
            let database_url = env::var("DATABASE_URL")
                .expect("DATABASE_URL must be set");
            let pool = db::new_pool(database_url);
            let addr = SyncArbiter::start(4, move || DbExecutor(pool.clone()));
            app::AppState{db: addr}
        })
        // register server handlers and start test server
        .start(|app| {
            app
                .resource("/api/questions", |r| r.method(http::Method::GET).f(routes::questions::get_all));
        })
    }
}
