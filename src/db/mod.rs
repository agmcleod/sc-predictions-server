use actix::{Actor, SyncContext};

pub mod models;
pub mod pool;
use self::pool::Pool;

pub struct DbExecutor(pub Pool);

impl Actor for DbExecutor {
    type Context = SyncContext<Self>;
}
