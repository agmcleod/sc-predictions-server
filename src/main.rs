extern crate actix;
extern crate actix_web;
extern crate chrono;
extern crate env_logger;
extern crate futures;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate log;
extern crate radix;
extern crate rand;
extern crate serde;
extern crate serde_json;
#[macro_use]
extern crate serde_derive;
extern crate postgres;
extern crate postgres_mapper;
#[macro_use]
extern crate postgres_mapper_derive;
extern crate dotenv;
extern crate r2d2;
extern crate r2d2_postgres;
extern crate uuid;

use std::env;

use actix::{Addr, SyncArbiter, System};
use actix_web::server;
use dotenv::dotenv;

mod app;
mod db;
mod errors;
mod routes;
mod utils;

use app::create_app;
use db::DbExecutor;

fn main() {
    dotenv().ok();
    env::set_var("RUST_LOG", "actix_web=info");
    env_logger::init();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");

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
    use super::*;
    use actix_web::{http, test::TestServer};
    use dotenv::dotenv;
    use r2d2::Pool;
    use r2d2_postgres::PostgresConnectionManager;
    use std::env;

    lazy_static! {
        pub static ref POOL: Pool<PostgresConnectionManager> = {
            dotenv().ok();
            let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
            db::new_pool(database_url)
        };
    }

    pub fn get_server() -> TestServer {
        match env_logger::try_init() {
            Err(_) => println!("Failed to init env logger"),
            _ => {}
        }
        dotenv().ok();
        TestServer::build_with_state(|| {
            let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
            let pool = db::new_pool(database_url);
            let addr = SyncArbiter::start(4, move || DbExecutor(pool.clone()));
            app::AppState { db: addr }
        })
        // register server handlers and start test server
        .start(|app| {
            app.resource("/api/questions", |r| {
                r.method(http::Method::GET).f(routes::questions::get_all)
            })
            .resource("/api/games", |r| r.post().with(routes::games::create));
        })
    }
}
