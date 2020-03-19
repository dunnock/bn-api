use db::dev::TestProject;
use db::prelude::*;

#[test]
fn find_all() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let mut domain_event_publisher = project.create_domain_event_publisher().finish();
    domain_event_publisher
        .update_last_domain_event_seq(1, connection)
        .unwrap();
    let domain_event_publisher = DomainEventPublisher::find(domain_event_publisher.id, connection).unwrap();
    let mut domain_event_publisher2 = project.create_domain_event_publisher().finish();
    domain_event_publisher2
        .update_last_domain_event_seq(2, connection)
        .unwrap();
    let domain_event_publisher2 = DomainEventPublisher::find(domain_event_publisher2.id, connection).unwrap();
    assert_eq!(
        DomainEventPublisher::find_all(connection).unwrap(),
        vec![domain_event_publisher.clone(), domain_event_publisher2.clone()]
    );

    domain_event_publisher2.delete(connection).unwrap();
    assert_eq!(
        DomainEventPublisher::find_all(connection).unwrap(),
        vec![domain_event_publisher]
    );
}

#[test]
fn update_last_domain_event_seq() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let mut domain_event_publisher = project.create_domain_event_publisher().finish();
    domain_event_publisher
        .update_last_domain_event_seq(1, connection)
        .unwrap();
    let mut domain_event_publisher = DomainEventPublisher::find(domain_event_publisher.id, connection).unwrap();
    assert_eq!(domain_event_publisher.last_domain_event_seq, Some(1));

    domain_event_publisher
        .update_last_domain_event_seq(2, connection)
        .unwrap();
    let domain_event_publisher = DomainEventPublisher::find(domain_event_publisher.id, connection).unwrap();
    assert_eq!(domain_event_publisher.last_domain_event_seq, Some(2));
}

#[test]
fn delete() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let domain_event_publisher = project.create_domain_event_publisher().finish();
    assert_eq!(DomainEventPublisher::find_all(connection).unwrap().len(), 1);

    domain_event_publisher.delete(connection).unwrap();
    assert_eq!(DomainEventPublisher::find_all(connection).unwrap().len(), 0);
}

#[test]
fn publish() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let organization = project.create_organization().with_fees().finish();
    let event = project
        .create_event()
        .with_organization(&organization)
        .with_ticket_pricing()
        .finish();
    let user = project.create_user().finish();
    project
        .create_order()
        .for_event(&event)
        .for_user(&user)
        .is_paid()
        .finish();
    let domain_event_publisher = DomainEventPublisher::create(
        None,
        vec![DomainEventTypes::OrderCompleted],
        "http://localhost:7644/webhook".to_string(),
    )
    .commit(connection)
    .unwrap();

    let publisher_data = DomainEvent::find_by_type(DomainEventTypes::OrderCompleted, connection).unwrap();

    assert_eq!(publisher_data.len(), 1);
    let domain_event = &publisher_data[0];
    domain_event_publisher
        .claim_for_publishing(&domain_event, connection)
        .unwrap();
}

#[test]
fn create() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let webhook = "http://localhost:7644/webhook".to_string();
    let organization = project.create_organization().finish();
    let domain_event_publisher = DomainEventPublisher::create(
        Some(organization.id),
        vec![DomainEventTypes::TransferTicketStarted],
        webhook.clone(),
    )
    .commit(connection)
    .unwrap();

    assert_eq!(domain_event_publisher.organization_id, Some(organization.id));
    assert_eq!(
        domain_event_publisher.event_types,
        vec![DomainEventTypes::TransferTicketStarted]
    );
    assert_eq!(domain_event_publisher.webhook_url, webhook);
    assert_eq!(domain_event_publisher.import_historic_events, false);
}

#[test]
fn find() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let organization = project.create_organization().finish();
    let domain_event_publisher = DomainEventPublisher::create(
        Some(organization.id),
        vec![DomainEventTypes::TransferTicketStarted],
        "http://localhost:7644/webhook".to_string(),
    )
    .commit(connection)
    .unwrap();
    let found_publisher = DomainEventPublisher::find(domain_event_publisher.id, connection).unwrap();
    assert_eq!(found_publisher, domain_event_publisher);
}

#[test]
fn update() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let organization = project.create_organization().finish();
    let domain_event_publisher = DomainEventPublisher::create(
        Some(organization.id),
        vec![DomainEventTypes::TransferTicketStarted],
        "http://localhost:7644/webhook".to_string(),
    )
    .commit(connection)
    .unwrap();

    let new_webhook_url = "http://localhost:7699/webhook".to_string();
    let parameters = DomainEventPublisherEditableAttributes {
        webhook_url: Some(new_webhook_url.clone()),
        import_historic_events: Some(false),
    };
    let domain_event_publisher = domain_event_publisher.update(&parameters, connection).unwrap();

    assert_eq!(domain_event_publisher.webhook_url, new_webhook_url);
    assert_eq!(domain_event_publisher.import_historic_events, false);
}

#[test]
fn acquire_lock() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let organization = project.create_organization().finish();
    let mut domain_event_publisher = DomainEventPublisher::create(
        Some(organization.id),
        vec![DomainEventTypes::TransferTicketStarted],
        "http://localhost:7644/webhook".to_string(),
    )
    .commit(connection)
    .unwrap();

    assert!(domain_event_publisher.acquire_lock(60, connection).is_ok());

    // pretend this is from another thread
    let mut domain_event_publisher_alias = DomainEventPublisher::find(domain_event_publisher.id, connection).unwrap();

    assert!(domain_event_publisher_alias.acquire_lock(60, connection).is_err());
}

#[test]
fn release_lock() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let organization = project.create_organization().finish();
    let mut domain_event_publisher = DomainEventPublisher::create(
        Some(organization.id),
        vec![DomainEventTypes::TransferTicketStarted],
        "http://localhost:7644/webhook".to_string(),
    )
    .commit(connection)
    .unwrap();

    // pretend this is from another thread
    let mut domain_event_publisher_alias = DomainEventPublisher::find(domain_event_publisher.id, connection).unwrap();

    assert!(domain_event_publisher.acquire_lock(60, connection).is_ok());

    assert!(domain_event_publisher_alias.acquire_lock(60, connection).is_err());

    assert!(domain_event_publisher.release_lock(connection).is_ok());

    assert!(domain_event_publisher_alias.acquire_lock(60, connection).is_ok());
}

#[test]
fn renew_lock() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let organization = project.create_organization().finish();
    let mut domain_event_publisher = DomainEventPublisher::create(
        Some(organization.id),
        vec![DomainEventTypes::TransferTicketStarted],
        "http://localhost:7644/webhook".to_string(),
    )
    .commit(connection)
    .unwrap();

    assert!(domain_event_publisher.acquire_lock(60, connection).is_ok());

    assert!(domain_event_publisher.renew_lock(60, connection).is_ok());
}
