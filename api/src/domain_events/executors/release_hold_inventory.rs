use crate::db::Connection;
use crate::domain_events::executor_future::ExecutorFuture;
use crate::domain_events::routing::DomainActionExecutor;
use crate::errors::*;
use bigneon_db::prelude::*;
use chrono::prelude::*;
use futures::future;
use log::Level::Error;

pub struct ReleaseHoldInventoryExecutor {}

impl DomainActionExecutor for ReleaseHoldInventoryExecutor {
    fn execute(&self, action: DomainAction, conn: Connection) -> ExecutorFuture {
        match self.perform_job(&action, &conn) {
            Ok(_) => ExecutorFuture::new(action, conn, Box::pin(future::ok(()))),
            Err(e) => {
                jlog!(Error, "Release hold inventory action failed", {"action_id": action.id, "main_table_id": action.main_table_id, "error": e.to_string()});
                ExecutorFuture::new(action, conn, Box::pin(future::err(e)))
            }
        }
    }
}

impl ReleaseHoldInventoryExecutor {
    pub fn new() -> ReleaseHoldInventoryExecutor {
        ReleaseHoldInventoryExecutor {}
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
            Tables::Holds => {
                let hold = Hold::find(id, conn)?;
                if let Some(end_at) = hold.end_at {
                    if end_at > Utc::now().naive_utc() {
                        return Err(
                            ApplicationError::new("Hold must have ended to release inventory".to_string()).into(),
                        );
                    }
                    let (total, remaining) = hold.quantity(conn)?;
                    if remaining > 0 {
                        let sold_quantity = total - remaining;
                        hold.set_quantity(None, sold_quantity, conn)?;
                        DomainEvent::create(
                            DomainEventTypes::HoldAutomaticallyReleased,
                            format!("Hold {} released", hold.name),
                            Tables::Holds,
                            Some(hold.id),
                            None,
                            Some(json!(&hold)),
                        )
                        .commit(conn)?;
                    }
                }
            }
            _ => return Err(ApplicationError::new("Table not supported".to_string()).into()),
        }
        Ok(())
    }
}
