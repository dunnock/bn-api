use crate::database::Connection;
use crate::domain_events::executor_future::ExecutorFuture;
use crate::domain_events::routing::DomainActionExecutor;
use crate::errors::*;
use db::prelude::*;
use futures::future;
use log::Level::Error;

pub struct FinalizeSettlementsExecutor {}

impl DomainActionExecutor for FinalizeSettlementsExecutor {
    fn execute(&self, action: DomainAction, conn: Connection) -> ExecutorFuture {
        match self.perform_job(&conn) {
            Ok(_) => ExecutorFuture::new(action, conn, Box::pin(future::ok(()))),
            Err(e) => {
                jlog!(Error, "Finalize settlements action failed", {"action_id": action.id, "main_table_id": action.main_table_id, "error": e.to_string()});
                ExecutorFuture::new(action, conn, Box::pin(future::err(e)))
            }
        }
    }
}

impl FinalizeSettlementsExecutor {
    pub fn new() -> FinalizeSettlementsExecutor {
        FinalizeSettlementsExecutor {}
    }

    pub fn perform_job(&self, conn: &Connection) -> Result<(), ApiError> {
        let conn = conn.get();
        Settlement::finalize_settlements(conn)?;
        Settlement::create_next_finalize_settlements_domain_action(conn)?;

        Ok(())
    }
}
