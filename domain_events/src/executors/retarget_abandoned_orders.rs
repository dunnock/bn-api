use crate::executor_future::ExecutorFuture;
use crate::routing::DomainActionExecutor;
use bigneon_db_connections::Connection;
use bigneon_errors::*;
use bigneon_db::prelude::*;
use futures::future;
use log::Level::Error;
use serde_json::json;
use logging::jlog;

pub struct RetargetAbandonedOrdersExecutor {}

impl DomainActionExecutor for RetargetAbandonedOrdersExecutor {
    fn execute(&self, action: DomainAction, conn: Connection) -> ExecutorFuture {
        match self.perform_job(&conn) {
            Ok(_) => ExecutorFuture::new(action, conn, Box::new(future::ok(()))),
            Err(e) => {
                jlog!(Error, "Retargeting abandoned orders action failed", {"action_id": action.id, "main_table_id": action.main_table_id, "error": e.to_string()});
                ExecutorFuture::new(action, conn, Box::new(future::err(e)))
            }
        }
    }
}

impl RetargetAbandonedOrdersExecutor {
    pub fn new() -> RetargetAbandonedOrdersExecutor {
        RetargetAbandonedOrdersExecutor {}
    }

    pub fn perform_job(&self, conn: &Connection) -> Result<(), BigNeonError> {
        let conn = conn.get();
        Order::retarget_abandoned_carts(conn)?;

        Order::create_next_retarget_abandoned_cart_domain_action(conn)?;

        Ok(())
    }
}
