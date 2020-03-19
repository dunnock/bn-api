use crate::config::Config;
use crate::database::Connection;
use crate::domain_events::executor_future::ExecutorFuture;
use crate::domain_events::routing::DomainActionExecutor;
use crate::errors::*;
use db::prelude::*;
use futures::future;
use log::Level::Error;

pub struct ProcessSettlementReportExecutor {
    config: Config,
}

impl DomainActionExecutor for ProcessSettlementReportExecutor {
    fn execute(&self, action: DomainAction, conn: Connection) -> ExecutorFuture {
        match self.perform_job(&action, &conn) {
            Ok(_) => ExecutorFuture::new(action, conn, Box::pin(future::ok(()))),
            Err(e) => {
                jlog!(Error, "Process transfer drip action failed", {"action_id": action.id, "main_table_id": action.main_table_id, "error": e.to_string()});
                ExecutorFuture::new(action, conn, Box::pin(future::err(e)))
            }
        }
    }
}

impl ProcessSettlementReportExecutor {
    pub fn new(config: Config) -> ProcessSettlementReportExecutor {
        ProcessSettlementReportExecutor { config }
    }

    pub fn perform_job(&self, action: &DomainAction, conn: &Connection) -> Result<(), ApiError> {
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
            Tables::Organizations => {
                let organization = Organization::find(id, conn)?;
                if organization.can_process_settlements(conn)? {
                    Settlement::process_settlement_for_organization(
                        &organization,
                        self.config.settlement_period_in_days,
                        conn,
                    )?;
                }
                organization
                    .create_next_settlement_processing_domain_action(self.config.settlement_period_in_days, conn)?;
            }
            _ => return Err(ApplicationError::new("Table not supported".to_string()).into()),
        };

        Ok(())
    }
}
