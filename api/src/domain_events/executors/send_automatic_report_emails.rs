use crate::communications::mailers;
use crate::config::Config;
use crate::database::Connection;
use crate::domain_events::executor_future::ExecutorFuture;
use crate::domain_events::routing::DomainActionExecutor;
use crate::errors::*;
use crate::models::*;
use db::prelude::*;
use futures::future;
use log::Level::Error;

pub struct SendAutomaticReportEmailsExecutor {
    config: Config,
}

impl DomainActionExecutor for SendAutomaticReportEmailsExecutor {
    fn execute(&self, action: DomainAction, conn: Connection) -> ExecutorFuture {
        match self.perform_job(&conn) {
            Ok(_) => ExecutorFuture::new(action, conn, Box::pin(future::ok(()))),
            Err(e) => {
                jlog!(Error, "Update send automatic report emails failed", {"action_id": action.id, "main_table_id": action.main_table_id, "error": e.to_string()});
                ExecutorFuture::new(action, conn, Box::pin(future::err(e)))
            }
        }
    }
}

impl SendAutomaticReportEmailsExecutor {
    pub fn new(config: Config) -> SendAutomaticReportEmailsExecutor {
        SendAutomaticReportEmailsExecutor { config }
    }

    pub fn perform_job(&self, conn: &Connection) -> Result<(), ApiError> {
        let conn = conn.get();

        for (report_type, events) in Report::find_event_reports_for_processing(conn)? {
            match report_type {
                ReportTypes::TicketCounts => {
                    for event in events {
                        let subscribers = EventReportSubscriber::find_all(event.id, report_type, conn)?;
                        let ticket_count_report: TicketCountReport =
                            Report::ticket_count_report(Some(event.id), Some(event.organization_id), conn)?.into();

                        for subscriber in subscribers {
                            if let Err(error) = mailers::reports::ticket_counts(
                                subscriber.email.clone(),
                                &event,
                                &ticket_count_report,
                                &self.config,
                                conn,
                            ) {
                                jlog!(Error, "Failed to send report to subscriber", {"report_type": report_type, "email": subscriber.email, "event_id": event.id, "error": error.to_string()});
                            }
                        }
                    }
                }
            }
        }

        let next_action = Report::create_next_automatic_report_domain_action(conn);
        match next_action {
            Err(err) if err.error_code == ErrorCode::AlreadyScheduledError => Ok(()),
            r => r,
        }?;

        Ok(())
    }
}
