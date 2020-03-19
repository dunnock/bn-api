use db::dev::TestProject;
use db::models::*;
use db::utils::errors::ErrorCode::ValidationError;

#[test]
fn destroy() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let user = project.create_user().finish();
    let event = project.create_event().finish();
    assert_eq!(
        DomainEvent::find(
            Tables::Events,
            Some(event.id),
            Some(DomainEventTypes::EventReportSubscriberDeleted),
            connection,
        )
        .unwrap()
        .len(),
        0
    );

    let event_report_subscriber = project.create_event_report_subscriber().with_event(&event).finish();
    assert!(event_report_subscriber.destroy(Some(user.id), connection).is_ok());
    assert!(EventReportSubscriber::find(event_report_subscriber.id, connection).is_err());
    let domain_events = DomainEvent::find(
        Tables::Events,
        Some(event.id),
        Some(DomainEventTypes::EventReportSubscriberDeleted),
        connection,
    )
    .unwrap();
    assert_eq!(domain_events.len(), 1);
    assert_eq!(
        domain_events[0].event_data,
        Some(
            json!({"email": event_report_subscriber.email, "event_report_subscriber_id": event_report_subscriber.id })
        )
    )
}

#[test]
fn find() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let event_report_subscriber = project.create_event_report_subscriber().finish();
    assert_eq!(
        event_report_subscriber,
        EventReportSubscriber::find(event_report_subscriber.id, connection).unwrap()
    );
}

#[test]
fn create() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let user = project.create_user().finish();
    let event = project.create_event().finish();
    let email = "subscriber@tari.com".to_string();
    assert_eq!(
        DomainEvent::find(
            Tables::Events,
            Some(event.id),
            Some(DomainEventTypes::EventReportSubscriberCreated),
            connection,
        )
        .unwrap()
        .len(),
        0
    );

    let event_report_subscriber = EventReportSubscriber::create(event.id, ReportTypes::TicketCounts, email.clone())
        .commit(Some(user.id), connection)
        .unwrap();

    assert_eq!(event_report_subscriber.event_id, event.id);
    assert_eq!(event_report_subscriber.report_type, ReportTypes::TicketCounts);
    assert_eq!(event_report_subscriber.email, email);
    let domain_events = DomainEvent::find(
        Tables::Events,
        Some(event.id),
        Some(DomainEventTypes::EventReportSubscriberCreated),
        connection,
    )
    .unwrap();
    assert_eq!(domain_events.len(), 1);
    assert_eq!(
        domain_events[0].event_data,
        Some(
            json!({"email": event_report_subscriber.email, "event_report_subscriber_id": event_report_subscriber.id })
        )
    )
}

#[test]
pub fn create_with_validation_errors() {
    let project = TestProject::new();
    let user = project.create_user().finish();
    let event = project.create_event().finish();
    let result = EventReportSubscriber::create(event.id, ReportTypes::TicketCounts, "invalid".to_string())
        .commit(Some(user.id), project.get_connection());

    match result {
        Ok(_) => {
            panic!("Expected validation error");
        }
        Err(error) => match &error.error_code {
            ValidationError { errors } => {
                assert!(errors.contains_key("email"));
                assert_eq!(errors["email"].len(), 1);
                assert_eq!(errors["email"][0].code, "email");
                assert_eq!(
                    &errors["email"][0].message.clone().unwrap().into_owned(),
                    "Email is invalid"
                );
            }
            _ => panic!("Expected validation error"),
        },
    }
}

#[test]
fn find_all() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let event = project.create_event().finish();
    let event2 = project.create_event().finish();
    let event3 = project.create_event().finish();
    let email = "subscriber-1@tari.com".to_string();
    let email2 = "subscriber-2@tari.com".to_string();
    let event_report_subscriber = project
        .create_event_report_subscriber()
        .with_event(&event)
        .with_email(&email)
        .finish();
    let event_report_subscriber2 = project
        .create_event_report_subscriber()
        .with_event(&event2)
        .with_email(&email)
        .finish();
    let event_report_subscriber3 = project
        .create_event_report_subscriber()
        .with_event(&event)
        .with_email(&email2)
        .finish();

    assert_eq!(
        EventReportSubscriber::find_all(event.id, ReportTypes::TicketCounts, connection).unwrap(),
        vec![event_report_subscriber, event_report_subscriber3]
    );
    assert_eq!(
        EventReportSubscriber::find_all(event2.id, ReportTypes::TicketCounts, connection).unwrap(),
        vec![event_report_subscriber2]
    );
    assert!(
        EventReportSubscriber::find_all(event3.id, ReportTypes::TicketCounts, connection)
            .unwrap()
            .is_empty()
    );
}
