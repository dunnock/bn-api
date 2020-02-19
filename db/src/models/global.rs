use diesel::prelude::*;
use models::*;
use utils::errors::DatabaseError;

pub fn schedule_domain_actions(conn: &PgConnection) -> Result<(), DatabaseError> {
    // Settlements weekly domain event
    if DomainAction::upcoming_domain_action(None, None, DomainActionTypes::SendAutomaticReportEmails, conn)?.is_none() {
        Report::create_next_automatic_report_domain_action(conn)?;
    }

    if DomainAction::upcoming_domain_action(None, None, DomainActionTypes::RetargetAbandonedOrders, conn)?.is_none() {
        Order::create_next_retarget_abandoned_cart_domain_action(conn)?;
    }

    Ok(())
}
