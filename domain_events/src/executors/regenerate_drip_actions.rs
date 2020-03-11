use crate::executor_future::ExecutorFuture;
use crate::routing::DomainActionExecutor;
use bigneon_config::Config;
use bigneon_db_connections::Connection;
use bigneon_errors::*;
use bigneon_db::prelude::*;
use futures::future;
use log::Level::Error;
use serde_json::json;
use logging::jlog;

pub struct RegenerateDripActionsExecutor {
    config: Config,
}

impl DomainActionExecutor for RegenerateDripActionsExecutor {
    fn execute(&self, action: DomainAction, conn: Connection) -> ExecutorFuture {
        match self.perform_job(&action, &conn) {
            Ok(_) => ExecutorFuture::new(action, conn, Box::new(future::ok(()))),
            Err(e) => {
                jlog!(Error, "Update genres action failed", {"action_id": action.id, "main_table_id": action.main_table_id, "error": e.to_string()});
                ExecutorFuture::new(action, conn, Box::new(future::err(e)))
            }
        }
    }
}

impl RegenerateDripActionsExecutor {
    pub fn new(config: Config) -> RegenerateDripActionsExecutor {
        RegenerateDripActionsExecutor { config }
    }

    pub fn perform_job(&self, action: &DomainAction, conn: &Connection) -> Result<(), BigNeonError> {
        let conn = conn.get();
        let id = action
            .main_table_id
            .clone()
            .ok_or(ApplicationError::new("No id supplied in the action".to_string()))?;

        match action
            .main_table
            .clone()
            .ok_or(ApplicationError::new("No table supplied in the action".to_string()))?
        {
            Tables::Events => {
                let event = Event::find(id, conn)?;
                event.clear_pending_drip_actions(conn)?;
                event.create_next_transfer_drip_action(self.config.environment, conn)?;
            }
            _ => return Err(ApplicationError::new("Table not supported".to_string()).into()),
        };

        Ok(())
    }
}
