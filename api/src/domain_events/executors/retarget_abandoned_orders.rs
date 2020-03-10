use crate::db::Connection;
use crate::domain_events::executor_future::ExecutorFuture;
use crate::domain_events::routing::DomainActionExecutor;
use crate::errors::*;
use bigneon_db::prelude::*;
use futures::future;
use log::Level::Error;

pub struct RetargetAbandonedOrdersExecutor {}

impl DomainActionExecutor for RetargetAbandonedOrdersExecutor {
    fn execute(&self, action: DomainAction, conn: Connection) -> ExecutorFuture {
        match self.perform_job(&conn) {
            Ok(_) => ExecutorFuture::new(action, conn, Box::pin(future::ok(()))),
            Err(e) => {
                jlog!(Error, "Retargeting abandoned orders action failed", {"action_id": action.id, "main_table_id": action.main_table_id, "error": e.to_string()});
                ExecutorFuture::new(action, conn, Box::pin(future::err(e)))
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
