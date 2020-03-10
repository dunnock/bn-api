use crate::communications::{mailers, smsers};
use crate::config::Config;
use crate::db::Connection;
use crate::domain_events::executor_future::ExecutorFuture;
use crate::domain_events::routing::DomainActionExecutor;
use crate::errors::*;
use crate::utils::ServiceLocator;
use bigneon_db::prelude::*;
use chrono::prelude::*;
use futures::future;
use log::Level::Error;

pub struct ProcessTransferDripEventExecutor {
    config: Config,
}

impl DomainActionExecutor for ProcessTransferDripEventExecutor {
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

impl ProcessTransferDripEventExecutor {
    pub fn new(config: Config) -> ProcessTransferDripEventExecutor {
        ProcessTransferDripEventExecutor { config }
    }

    pub fn perform_job(&self, action: &DomainAction, conn: &Connection) -> Result<(), BigNeonError> {
        let conn = conn.get();
        let id = action
            .main_table_id
            .clone()
            .ok_or(ApplicationError::new("No id supplied in the action".to_string()))?;

        let payload: ProcessTransferDripPayload = serde_json::from_value(action.payload.clone())?;

        match action
            .main_table
            .clone()
            .ok_or(ApplicationError::new("No table supplied in the action".to_string()))?
        {
            Tables::Events => {
                let event = Event::find(id, conn)?;
                if let Some(publish_date) = event.publish_date {
                    if publish_date <= Utc::now().naive_utc() {
                        for transfer in event.pending_transfers(conn)? {
                            transfer.create_drip_actions(&event, conn)?;
                        }
                    }
                }
                event.create_next_transfer_drip_action(self.config.environment, conn)?;
            }
            Tables::Transfers => {
                let event = Event::find(payload.event_id, conn)?;
                let transfer = Transfer::find(id, conn)?;
                let source_user = User::find(transfer.source_user_id, conn)?;
                if transfer.can_process_drips(conn)? {
                    match payload.source_or_destination {
                        SourceOrDestination::Source => {
                            if let Some(source_email) = source_user.email.clone() {
                                mailers::tickets::transfer_drip_reminder(
                                    source_email,
                                    &transfer,
                                    &event,
                                    SourceOrDestination::Source,
                                    &self.config,
                                    conn,
                                )?;
                            }
                            transfer.log_drip_domain_event(SourceOrDestination::Source, conn)?;
                        }
                        SourceOrDestination::Destination => {
                            if let (Some(transfer_message_type), Some(transfer_address)) =
                                (transfer.transfer_message_type, &transfer.transfer_address)
                            {
                                match transfer_message_type {
                                    TransferMessageType::Phone => {
                                        smsers::tickets::transfer_drip_reminder(
                                            transfer_address.clone(),
                                            &transfer,
                                            &event,
                                            &self.config,
                                            conn,
                                            &*ServiceLocator::new(&self.config)?.create_deep_linker()?,
                                        )?;
                                    }

                                    TransferMessageType::Email => {
                                        mailers::tickets::transfer_drip_reminder(
                                            transfer_address.clone(),
                                            &transfer,
                                            &event,
                                            SourceOrDestination::Destination,
                                            &self.config,
                                            conn,
                                        )?;
                                    }
                                }

                                transfer.log_drip_domain_event(SourceOrDestination::Destination, conn)?;
                            }
                        }
                    }
                }
            }
            _ => return Err(ApplicationError::new("Table not supported".to_string()).into()),
        };

        Ok(())
    }
}
