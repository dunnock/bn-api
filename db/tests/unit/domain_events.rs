use chrono::Utc;
use db::dev::TestProject;
use db::prelude::*;
use serde_json::Value;
use std::str::FromStr;
use uuid::Uuid;

#[test]
fn find_after_seq() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let initial_domain_event = DomainEvent::create(
        DomainEventTypes::EventArtistAdded,
        "First".to_string(),
        Tables::EventArtists,
        None,
        None,
        None,
    )
    .commit(connection)
    .unwrap();
    assert!(DomainEvent::find_after_seq(initial_domain_event.seq, 2, connection)
        .unwrap()
        .is_empty());

    let domain_event = DomainEvent::create(
        DomainEventTypes::EventArtistAdded,
        "First".to_string(),
        Tables::EventArtists,
        None,
        None,
        None,
    )
    .commit(connection)
    .unwrap();
    assert_eq!(
        DomainEvent::find_after_seq(initial_domain_event.seq, 2, connection).unwrap(),
        vec![domain_event.clone()]
    );
    assert!(DomainEvent::find_after_seq(domain_event.seq, 2, connection)
        .unwrap()
        .is_empty());

    let domain_event2 = DomainEvent::create(
        DomainEventTypes::EventArtistAdded,
        "First".to_string(),
        Tables::EventArtists,
        None,
        None,
        None,
    )
    .commit(connection)
    .unwrap();
    assert_eq!(
        DomainEvent::find_after_seq(initial_domain_event.seq, 2, connection).unwrap(),
        vec![domain_event.clone(), domain_event2.clone()]
    );
    assert_eq!(
        DomainEvent::find_after_seq(initial_domain_event.seq, 1, connection).unwrap(),
        vec![domain_event.clone()]
    );
    assert_eq!(
        DomainEvent::find_after_seq(domain_event.seq, 2, connection).unwrap(),
        vec![domain_event2.clone()]
    );
    assert!(DomainEvent::find_after_seq(domain_event2.seq, 2, connection)
        .unwrap()
        .is_empty());
}

#[test]
fn find_by_ids() {
    let project = TestProject::new();
    let connection = project.get_connection();

    let domain_event = DomainEvent::create(
        DomainEventTypes::EventArtistAdded,
        "Nothing to see here".to_string(),
        Tables::EventArtists,
        None,
        None,
        None,
    )
    .commit(connection)
    .unwrap();

    let domain_event2 = DomainEvent::create(
        DomainEventTypes::EventArtistAdded,
        "Nothing to see here".to_string(),
        Tables::EventArtists,
        None,
        None,
        None,
    )
    .commit(connection)
    .unwrap();

    assert_eq!(
        vec![domain_event.clone()],
        DomainEvent::find_by_ids(vec![domain_event.id], connection).unwrap()
    );
    assert_eq!(
        vec![domain_event2.clone()],
        DomainEvent::find_by_ids(vec![domain_event2.id], connection).unwrap()
    );
    assert_equiv!(
        DomainEvent::find_by_ids(vec![domain_event.id, domain_event2.id], connection).unwrap(),
        vec![domain_event, domain_event2]
    );
}

#[test]
fn serialize() {
    let project = TestProject::new();
    let conn = project.get_connection();

    let domain_event = DomainEvent::create(
        DomainEventTypes::EventArtistAdded,
        "Nothing to see here".to_string(),
        Tables::EventArtists,
        None,
        None,
        None,
    )
    .commit(conn)
    .unwrap();

    let json = json!(domain_event).to_string();
    let v: Value = serde_json::from_str(json.as_str()).unwrap();
    assert_eq!(v["event_type"], "EventArtistAdded");
    assert_eq!(v["display_text"], "Nothing to see here");
    assert_eq!(v["main_table"], "EventArtists");
}

#[test]
fn partial_ord() {
    let make_dummy_event = |id| DomainEvent {
        id,
        event_type: DomainEventTypes::EventArtistAdded,
        display_text: "".to_string(),
        event_data: None,
        main_table: Tables::EventArtists,
        main_id: None,
        created_at: Utc::now().naive_utc(),
        updated_at: Utc::now().naive_utc(),
        user_id: None,
        organization_id: None,
        seq: 0,
    };

    let high_id = "e2cf68a4-76bb-49e1-993c-2576a4fc1220";
    let low_id = "e2cf68a4-76bb-49e1-993c-2576a4fc1221";

    // Anti-symmetry
    let a = make_dummy_event(Uuid::from_str(low_id).unwrap());
    let b = make_dummy_event(Uuid::from_str(high_id).unwrap());

    assert!(a > b);
    assert!(!(a < b));

    let a = make_dummy_event(Uuid::from_str(high_id).unwrap());
    let b = make_dummy_event(Uuid::from_str(low_id).unwrap());

    assert!(a < b);
    assert!(!(a > b));
}

#[test]
fn commit() {
    let project = TestProject::new();
    let conn = project.get_connection();

    let domain_event = DomainEvent::create(
        DomainEventTypes::EventArtistAdded,
        "".to_string(),
        Tables::EventArtists,
        None,
        None,
        None,
    );

    let domain_action = domain_event.commit(conn).unwrap();

    assert!(!domain_action.id.is_nil());
}

#[test]
fn find() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let user = project.create_user().finish();
    let id = Uuid::new_v4();

    // Empty, no events
    assert!(DomainEvent::find(Tables::PaymentMethods, Some(id), None, connection)
        .unwrap()
        .is_empty());

    // New events
    let domain_event = DomainEvent::create(
        DomainEventTypes::PaymentMethodCreated,
        "Payment method was created".to_string(),
        Tables::PaymentMethods,
        Some(id),
        Some(user.id),
        Some("".into()),
    )
    .commit(connection)
    .unwrap();

    let domain_event2 = DomainEvent::create(
        DomainEventTypes::PaymentMethodUpdated,
        "Payment method was updated".to_string(),
        Tables::PaymentMethods,
        Some(id),
        Some(user.id),
        Some("".into()),
    )
    .commit(connection)
    .unwrap();

    assert_equiv!(
        DomainEvent::find(Tables::PaymentMethods, Some(id), None, connection).unwrap(),
        [domain_event.clone(), domain_event2.clone()]
    );
    assert!(DomainEvent::find(Tables::Payments, Some(id), None, connection)
        .unwrap()
        .is_empty());
    assert!(
        DomainEvent::find(Tables::PaymentMethods, Some(Uuid::new_v4()), None, connection,)
            .unwrap()
            .is_empty()
    );

    // Filtered by type
    assert_eq!(
        DomainEvent::find(
            Tables::PaymentMethods,
            Some(id),
            Some(DomainEventTypes::PaymentMethodCreated),
            connection,
        )
        .unwrap(),
        [domain_event]
    );
    assert_eq!(
        DomainEvent::find(
            Tables::PaymentMethods,
            Some(id),
            Some(DomainEventTypes::PaymentMethodUpdated),
            connection,
        )
        .unwrap(),
        [domain_event2]
    );
}
