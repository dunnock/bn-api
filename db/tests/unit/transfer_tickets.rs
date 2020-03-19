use db::dev::TestProject;
use db::models::*;
use db::utils::errors::ErrorCode::ValidationError;
use uuid::Uuid;

#[test]
fn create_commit() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let user = project.create_user().finish();
    project.create_order().for_user(&user).quantity(1).is_paid().finish();
    let ticket = TicketInstance::find_for_user(user.id, connection).unwrap().remove(0);

    let transfer = Transfer::create(user.id, Uuid::new_v4(), None, None, false)
        .commit(connection)
        .unwrap();
    let transfer_ticket = TransferTicket::create(ticket.id, transfer.id)
        .commit(connection)
        .unwrap();
    assert_eq!(transfer_ticket.ticket_instance_id, ticket.id);
    assert_eq!(transfer_ticket.transfer_id, transfer.id);
}

#[test]
fn pending_transfer() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let mut user = project.create_user().finish();
    user = user.add_role(Roles::Super, connection).unwrap();
    let user2 = project.create_user().finish();
    let event = project.create_event().with_tickets().finish();
    project
        .create_order()
        .for_event(&event)
        .for_user(&user)
        .is_paid()
        .finish();
    let user_tickets = TicketInstance::find_for_user(user.id, connection).unwrap();
    let ticket = &user_tickets[0];
    assert!(TransferTicket::pending_transfer(ticket.id, connection)
        .unwrap()
        .is_none());

    // With pending transfer
    let transfer = TicketInstance::create_transfer(&user, &[ticket.id], None, None, false, connection).unwrap();
    assert_eq!(
        TransferTicket::pending_transfer(ticket.id, connection).unwrap(),
        Some(transfer.clone())
    );

    // With cancelled transfer
    transfer.cancel(&user, None, connection).unwrap();
    assert!(TransferTicket::pending_transfer(ticket.id, connection)
        .unwrap()
        .is_none());

    // With completed direct transfer
    TicketInstance::direct_transfer(
        &user,
        &[ticket.id],
        "nowhere",
        TransferMessageType::Email,
        user2.id,
        connection,
    )
    .unwrap();
    assert!(TransferTicket::pending_transfer(ticket.id, connection)
        .unwrap()
        .is_none());

    // User 2 retransfers
    let transfer = TicketInstance::create_transfer(&user2, &[ticket.id], None, None, false, connection).unwrap();
    assert_eq!(
        TransferTicket::pending_transfer(ticket.id, connection).unwrap(),
        Some(transfer)
    );
}

#[test]
fn create_commit_with_validation_error() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let user = project.create_user().finish();
    let user2 = project.create_user().finish();
    project.create_order().for_user(&user).quantity(1).is_paid().finish();
    let ticket = TicketInstance::find_for_user(user.id, connection).unwrap().remove(0);

    let transfer = Transfer::create(user.id, Uuid::new_v4(), None, None, false)
        .commit(connection)
        .unwrap();
    TransferTicket::create(ticket.id, transfer.id)
        .commit(connection)
        .unwrap();

    // Active pending transfer already exists triggering validation errors
    let transfer2 = Transfer::create(user.id, Uuid::new_v4(), None, None, false)
        .commit(connection)
        .unwrap();
    let result = TransferTicket::create(ticket.id, transfer2.id).commit(connection);
    assert!(result.is_err());
    print!("{:?}", result);
    match result {
        Ok(_) => {
            panic!("Expected validation error");
        }
        Err(error) => match &error.error_code {
            ValidationError { errors } => {
                assert!(errors.contains_key("ticket_instance_id"));
                assert_eq!(errors["ticket_instance_id"].len(), 1);
                assert_eq!(errors["ticket_instance_id"][0].code, "too_many_pending_transfers");
                assert_eq!(
                    errors["ticket_instance_id"][0].message.clone().unwrap().into_owned(),
                    "An active pending transfer already exists for this ticket instance id"
                )
            }
            _ => panic!("Expected validation error"),
        },
    }

    // Original transfer is now complete so no issue
    assert!(transfer.complete(user2.id, None, connection).is_ok());
    TransferTicket::create(ticket.id, transfer2.id)
        .commit(connection)
        .unwrap();
}
