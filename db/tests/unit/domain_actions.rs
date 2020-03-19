use chrono::{Duration, Utc};
use db::dev::TestProject;
use db::prelude::*;
use uuid::Uuid;

#[test]
fn upcoming_domain_action() {
    let project = TestProject::new();
    let connection = project.get_connection();
    Report::create_next_automatic_report_domain_action(connection).unwrap();
    let upcoming_automatic_report_domain_action =
        DomainAction::upcoming_domain_action(None, None, DomainActionTypes::SendAutomaticReportEmails, connection)
            .unwrap();
    assert!(upcoming_automatic_report_domain_action.is_some());

    // Mark as done
    upcoming_automatic_report_domain_action
        .unwrap()
        .set_done(connection)
        .unwrap();
    let upcoming_automatic_report_domain_action =
        DomainAction::upcoming_domain_action(None, None, DomainActionTypes::SendAutomaticReportEmails, connection)
            .unwrap();
    assert!(upcoming_automatic_report_domain_action.is_none());
}

#[test]
fn commit() {
    let project = TestProject::new();
    let conn = project.get_connection();
    let domain_action = DomainAction::create(
        None,
        DomainActionTypes::Communication,
        None,
        serde_json::Value::Null,
        None,
        None,
    );

    let now = Utc::now().naive_utc();
    assert!(domain_action.scheduled_at <= now && domain_action.scheduled_at > now - Duration::hours(1));

    let domain_action = domain_action.commit(conn).unwrap();
    assert!(!domain_action.id.is_nil());
    assert_eq!(DomainActionTypes::Communication, domain_action.domain_action_type);
}

#[test]
fn new_scheduled_at() {
    let mut domain_action = DomainAction::create(
        None,
        DomainActionTypes::Communication,
        None,
        serde_json::Value::Null,
        None,
        None,
    );

    let new_scheduled_at = domain_action.scheduled_at + Duration::hours(1);
    let expires_at = domain_action.expires_at.clone();
    let blocked_until = domain_action.blocked_until.clone();

    domain_action.schedule_at(new_scheduled_at);
    assert_eq!(new_scheduled_at.timestamp(), domain_action.scheduled_at.timestamp());
    assert_eq!(60 * 60, domain_action.expires_at.timestamp() - expires_at.timestamp());
    assert_eq!(
        60 * 60,
        domain_action.blocked_until.timestamp() - blocked_until.timestamp()
    );
}

#[test]
fn find_stuck() {
    let project = TestProject::new();
    let conn = project.get_connection();

    let the_past = Utc::now().naive_utc() - Duration::hours(1);

    let stuck_example = project.create_domain_action().with_blocked_until(the_past).finish();
    // Create another domain action just to prove that find_stuck doesn't return everything
    let _ = project.create_domain_action().finish();

    let stuck_actions = DomainAction::find_stuck(conn).unwrap();

    assert_eq!(1, stuck_actions.len());
    assert_eq!(stuck_example.id, stuck_actions[0].id);
}

#[test]
fn find_by_resource() {
    let project = TestProject::new();
    let conn = project.get_connection();

    let main_table = Tables::Organizations;
    let main_table_id = Uuid::new_v4();
    let domain_action = project
        .create_domain_action()
        .with_main_table(main_table.clone())
        .with_main_table_id(main_table_id)
        .with_domain_action_type(DomainActionTypes::UpdateGenres)
        .with_status(DomainActionStatus::Pending)
        .finish();

    let pending_actions = DomainAction::find_by_resource(
        Some(main_table.clone()),
        Some(main_table_id),
        DomainActionTypes::UpdateGenres,
        DomainActionStatus::Pending,
        conn,
    )
    .unwrap();
    assert_eq!(1, pending_actions.len());
    assert_eq!(domain_action.id, pending_actions[0].id);

    domain_action.set_done(conn).unwrap();
    let pending_actions = DomainAction::find_by_resource(
        Some(main_table),
        Some(main_table_id),
        DomainActionTypes::UpdateGenres,
        DomainActionStatus::Pending,
        conn,
    )
    .unwrap();
    assert_eq!(0, pending_actions.len());
}

#[test]
fn find_pending() {
    let project = TestProject::new();
    let conn = project.get_connection();

    let pending_example = project
        .create_domain_action()
        .with_status(DomainActionStatus::Pending)
        .finish();

    let pending_actions = DomainAction::find_pending(None, conn).unwrap();

    assert_eq!(1, pending_actions.len());
    assert_eq!(pending_example.id, pending_actions[0].id);
}

#[test]
fn has_pending_action() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let id = Uuid::new_v4();

    // Empty, no action
    let result = DomainAction::has_pending_action(
        DomainActionTypes::BroadcastPushNotification,
        Tables::Events,
        id,
        connection,
    )
    .unwrap();
    assert!(!result);

    // New action
    let mut domain_action = DomainAction::create(
        None,
        DomainActionTypes::BroadcastPushNotification,
        None,
        json!(Vec::<u8>::new()),
        Some(Tables::Events),
        Some(id),
    );
    domain_action.scheduled_at = Utc::now().naive_utc().checked_add_signed(Duration::hours(1)).unwrap();
    domain_action.expires_at = Utc::now().naive_utc().checked_add_signed(Duration::days(1)).unwrap();

    domain_action.commit(connection).unwrap();

    let result = DomainAction::has_pending_action(
        DomainActionTypes::BroadcastPushNotification,
        Tables::Events,
        id,
        connection,
    )
    .unwrap();
    assert!(result);
}

#[test]
fn set_busy() {
    let project = TestProject::new();
    let conn = project.get_connection();

    let example = project.create_domain_action().finish();

    example.set_busy(9999, conn).unwrap();

    let updated = DomainAction::find(example.id, conn).unwrap();

    assert!(updated.blocked_until > Utc::now().naive_utc());

    let err = updated.set_busy(9999, conn).err().unwrap();
    assert_eq!(err.error_code, ErrorCode::ConcurrencyError);
}

#[test]
fn set_cancelled() {
    let project = TestProject::new();
    let conn = project.get_connection();

    let example = project
        .create_domain_action()
        .with_status(DomainActionStatus::Pending)
        .finish();
    example.set_cancelled(conn).unwrap();

    let updated = DomainAction::find(example.id, conn).unwrap();

    assert_eq!(DomainActionStatus::Cancelled, updated.status);
}

#[test]
fn set_done() {
    let project = TestProject::new();
    let conn = project.get_connection();

    let example = project
        .create_domain_action()
        .with_status(DomainActionStatus::Pending)
        .finish();
    example.set_done(conn).unwrap();

    let updated = DomainAction::find(example.id, conn).unwrap();

    assert_eq!(DomainActionStatus::Success, updated.status);
}

#[test]
fn set_failed() {
    let project = TestProject::new();
    let conn = project.get_connection();

    // Retry failure
    let example = project
        .create_domain_action()
        .with_status(DomainActionStatus::Pending)
        .finish();
    example.set_failed("test", conn).unwrap();

    let updated = DomainAction::find(example.id, conn).unwrap();

    assert_eq!("test", updated.last_failure_reason.unwrap());
    assert_eq!(DomainActionStatus::Pending, updated.status);
    assert_eq!(1, updated.attempt_count);

    // Exceeding max failures
    let example = project
        .create_domain_action()
        .with_status(DomainActionStatus::Pending)
        .with_attempt_count(1)
        .with_max_attempt_count(2)
        .finish();
    example.set_failed("test2", conn).unwrap();

    let updated = DomainAction::find(example.id, conn).unwrap();

    assert_eq!("test2", updated.last_failure_reason.unwrap());
    assert_eq!(DomainActionStatus::RetriesExceeded, updated.status);
    assert_eq!(2, updated.attempt_count);
    assert!(updated.blocked_until.timestamp() <= Utc::now().naive_utc().timestamp());
}

#[test]
fn set_errored() {
    let project = TestProject::new();
    let conn = project.get_connection();

    let example = project
        .create_domain_action()
        .with_status(DomainActionStatus::Pending)
        .finish();
    example.set_errored("test", conn).unwrap();

    let updated = DomainAction::find(example.id, conn).unwrap();

    assert_eq!("test", updated.last_failure_reason.unwrap());
    assert_eq!(DomainActionStatus::Errored, updated.status);
    assert_eq!(0, updated.attempt_count);
    assert!(updated.blocked_until.timestamp() <= Utc::now().naive_utc().timestamp());
}

#[test]
fn update() {
    let project = TestProject::new();
    let conn = project.get_connection();

    let example = project.create_domain_action().finish();

    let scheduled_at = Utc::now().naive_utc() + Duration::hours(1);
    let last_attempted_at = Utc::now().naive_utc() - Duration::hours(1);
    let blocked_until = Utc::now().naive_utc();
    let attributes = DomainActionEditableAttributes {
        scheduled_at: Some(scheduled_at),
        last_attempted_at: Some(last_attempted_at),
        attempt_count: Some(123),
        blocked_until,
    };
    example.update(&attributes, conn).unwrap();

    let updated = DomainAction::find(example.id, conn).unwrap();

    assert_eq!(scheduled_at.timestamp(), updated.scheduled_at.timestamp());
    assert_eq!(
        last_attempted_at.timestamp(),
        updated.last_attempted_at.unwrap().timestamp()
    );
    assert_eq!(123, updated.attempt_count);
    assert_eq!(blocked_until.timestamp(), updated.blocked_until.timestamp());
}
