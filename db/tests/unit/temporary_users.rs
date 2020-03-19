use db::dev::TestProject;
use db::prelude::*;
use uuid::Uuid;

#[test]
fn create() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let uuid = Uuid::new_v4();
    let email = "email@tari.com".to_string();
    let phone = "111111111111".to_string();
    let user = project.create_user().finish();

    let domain_events = DomainEvent::find(
        Tables::TemporaryUsers,
        Some(uuid),
        Some(DomainEventTypes::TemporaryUserCreated),
        connection,
    )
    .unwrap();
    assert_eq!(0, domain_events.len());

    let temporary_user = TemporaryUser::create(uuid, Some(email.clone()), Some(phone.clone()))
        .commit(user.id, connection)
        .unwrap();

    assert_eq!(temporary_user.id, uuid);
    assert_eq!(temporary_user.email, Some(email));
    assert_eq!(temporary_user.phone, Some(phone));

    let domain_events = DomainEvent::find(
        Tables::TemporaryUsers,
        Some(uuid),
        Some(DomainEventTypes::TemporaryUserCreated),
        connection,
    )
    .unwrap();
    assert_eq!(1, domain_events.len());
}

#[test]
fn associate_user() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let user = project.create_user().finish();
    let temporary_user = TemporaryUser::create(Uuid::new_v4(), None, None)
        .commit(user.id, connection)
        .unwrap();
    assert!(TemporaryUser::find_by_user_id(user.id, connection).unwrap().is_empty());

    // Results appear once linked
    temporary_user.associate_user(user.id, connection).unwrap();
    assert_eq!(
        TemporaryUser::find_by_user_id(user.id, connection).unwrap(),
        vec![temporary_user.clone()]
    );

    // Subsequent associate calls do not link cause errors or additional links
    temporary_user.associate_user(user.id, connection).unwrap();
    assert_eq!(
        TemporaryUser::find_by_user_id(user.id, connection).unwrap(),
        vec![temporary_user]
    );
}

#[test]
fn find_or_build_from_transfer() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let user = project.create_user().finish();
    let event = project.create_event().with_ticket_pricing().finish();
    project
        .create_order()
        .for_user(&user)
        .for_event(&event)
        .quantity(4)
        .is_paid()
        .finish();
    let ticket = &TicketInstance::find_for_user(user.id, connection).unwrap()[0];
    let ticket2 = &TicketInstance::find_for_user(user.id, connection).unwrap()[1];
    let ticket3 = &TicketInstance::find_for_user(user.id, connection).unwrap()[2];
    let ticket4 = &TicketInstance::find_for_user(user.id, connection).unwrap()[3];

    let email = "test@tari.com".to_string();
    let phone = "12345678901".to_string();

    let transfer = TicketInstance::create_transfer(
        &user,
        &[ticket.id],
        Some(&email),
        Some(TransferMessageType::Email),
        false,
        connection,
    )
    .unwrap();
    let transfer2 = TicketInstance::create_transfer(
        &user,
        &[ticket2.id],
        Some("testing@tari.com"),
        Some(TransferMessageType::Email),
        false,
        connection,
    )
    .unwrap();
    let transfer3 = TicketInstance::create_transfer(
        &user,
        &[ticket3.id],
        Some(&email),
        Some(TransferMessageType::Email),
        false,
        connection,
    )
    .unwrap();
    let transfer4 = TicketInstance::create_transfer(
        &user,
        &[ticket4.id],
        Some(&phone),
        Some(TransferMessageType::Phone),
        false,
        connection,
    )
    .unwrap();

    let temporary_user = TemporaryUser::find_or_build_from_transfer(&transfer, connection)
        .unwrap()
        .unwrap();
    assert_eq!(Some(email.clone()), temporary_user.email);

    let temporary_user2 = TemporaryUser::find_or_build_from_transfer(&transfer2, connection)
        .unwrap()
        .unwrap();
    assert_ne!(Some(email.clone()), temporary_user2.email);

    let temporary_user3 = TemporaryUser::find_or_build_from_transfer(&transfer3, connection)
        .unwrap()
        .unwrap();
    assert_eq!(Some(email.clone()), temporary_user3.email);
    assert_eq!(temporary_user, temporary_user3);

    let temporary_user4 = TemporaryUser::find_or_build_from_transfer(&transfer4, connection)
        .unwrap()
        .unwrap();
    assert_eq!(Some(phone.clone()), temporary_user4.phone);
}

#[test]
fn find_by_user_id() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let user = project.create_user().finish();
    let user2 = project.create_user().finish();
    let temporary_user = TemporaryUser::create(Uuid::new_v4(), None, None)
        .commit(user.id, connection)
        .unwrap();
    assert!(TemporaryUser::find_by_user_id(user.id, connection).unwrap().is_empty());
    assert!(TemporaryUser::find_by_user_id(user2.id, connection).unwrap().is_empty());

    temporary_user.associate_user(user.id, connection).unwrap();
    assert_eq!(
        TemporaryUser::find_by_user_id(user.id, connection).unwrap(),
        vec![temporary_user]
    );
    assert!(TemporaryUser::find_by_user_id(user2.id, connection).unwrap().is_empty());
}

#[test]
fn find() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let user = project.create_user().finish();
    let temporary_user = TemporaryUser::create(Uuid::new_v4(), None, None)
        .commit(user.id, connection)
        .unwrap();
    let found_temporary_user = TemporaryUser::find(temporary_user.id, connection).unwrap();
    assert_eq!(found_temporary_user, temporary_user);
}
