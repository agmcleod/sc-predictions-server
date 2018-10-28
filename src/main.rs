extern crate actix;
extern crate actix_web;
extern crate chrono;
extern crate futures;
extern crate env_logger;
#[macro_use] extern crate log;
extern crate serde;
#[macro_use] extern crate serde_derive;
extern crate postgres;
extern crate postgres_mapper;
#[macro_use] extern crate postgres_mapper_derive;
extern crate dotenv;
extern crate r2d2;
extern crate r2d2_postgres;

use std::env;

use actix::{Addr, SyncArbiter};
use actix_web::{server};
use dotenv::dotenv;

mod app;
mod errors;
mod routes;
mod db;

use app::create_app;
use db::{DbExecutor, pool::Pool};

fn main() {
    dotenv().ok();
    std::env::set_var("RUST_LOG", "actix_web=info");
    env_logger::init();

    let database_url = env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");

    let pool = Pool::from_url(&database_url);

    if let Ok(pool) = pool {
        // start arbiter with 4 threads
        // usage of move for the closure lets it take ownership of pool, before cloning it
        let address: Addr<DbExecutor> = SyncArbiter::start(4, move || DbExecutor(pool.clone()));

        server::new(move || {
            create_app(address.clone())
        })
        .bind("0.0.0.0:8080").expect("Can not bind to :8080")
        .run();
    } else {
        panic!("Could not start pool");
    }

}
