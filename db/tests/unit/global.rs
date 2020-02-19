use bigneon_db::dev::TestProject;
use bigneon_db::models::*;
use diesel::prelude::*;

#[test]
fn schedule_domain_actions() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let domain_actions = domain_actions_pending(DomainActionTypes::SendAutomaticReportEmails, connection);
    assert_eq!(0, domain_actions.len());
    let domain_actions = domain_actions_pending(DomainActionTypes::RetargetAbandonedOrders, connection);
    assert_eq!(0, domain_actions.len());

    // Schedule domain action
    global::schedule_domain_actions(connection).unwrap();
    let domain_actions = domain_actions_pending(DomainActionTypes::SendAutomaticReportEmails, connection);
    assert_eq!(1, domain_actions.len());
    let domain_actions = domain_actions_pending(DomainActionTypes::RetargetAbandonedOrders, connection);
    assert_eq!(1, domain_actions.len());

    // No change since action exists
    global::schedule_domain_actions(connection).unwrap();
    let domain_actions = domain_actions_pending(DomainActionTypes::SendAutomaticReportEmails, connection);
    assert_eq!(1, domain_actions.len());
    // Delete action
    domain_actions[0].set_done(connection).unwrap();
    let domain_actions = domain_actions_pending(DomainActionTypes::RetargetAbandonedOrders, connection);
    assert_eq!(1, domain_actions.len());
    // Delete action
    domain_actions[0].set_done(connection).unwrap();

    let domain_actions = domain_actions_pending(DomainActionTypes::SendAutomaticReportEmails, connection);
    assert_eq!(0, domain_actions.len());
    let domain_actions = domain_actions_pending(DomainActionTypes::RetargetAbandonedOrders, connection);
    assert_eq!(0, domain_actions.len());

    // Added back as it was no longer there (the job creates the next domain action normally)
    global::schedule_domain_actions(connection).unwrap();
    let domain_actions = domain_actions_pending(DomainActionTypes::SendAutomaticReportEmails, connection);
    assert_eq!(1, domain_actions.len());
    let domain_actions = domain_actions_pending(DomainActionTypes::RetargetAbandonedOrders, connection);
    assert_eq!(1, domain_actions.len());
}

fn domain_actions_pending(domain_action_type: DomainActionTypes, connection: &PgConnection) -> Vec<DomainAction> {
    DomainAction::find_by_resource(None, None, domain_action_type, DomainActionStatus::Pending, connection).unwrap()
}
