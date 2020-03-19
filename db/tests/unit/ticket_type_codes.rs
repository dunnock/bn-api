use db::dev::TestProject;
use db::prelude::*;
use db::utils::errors::ErrorCode::ValidationError;

#[test]
fn create() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let event = project.create_event().with_ticket_pricing().finish();
    let code = project.create_code().with_event(&event).finish();
    let ticket_type = &event.ticket_types(true, None, connection).unwrap()[0];

    let ticket_type_code = TicketTypeCode::create(ticket_type.id, code.id)
        .commit(connection)
        .unwrap();

    assert_eq!(
        ticket_type_code.ticket_type_id, ticket_type.id,
        "TicketType foreign key does not match"
    );
    assert_eq!(ticket_type_code.code_id, code.id, "Code foreign key does not match");
}

#[test]
fn destroy_multiple() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let event = project
        .create_event()
        .with_ticket_pricing()
        .with_ticket_type_count(3)
        .finish();
    let ticket_types = event.ticket_types(true, None, &connection).unwrap();
    let ticket_type = &ticket_types[0];
    let ticket_type2 = &ticket_types[1];
    let ticket_type3 = &ticket_types[2];
    let code = project.create_code().with_event(&event).finish();

    TicketTypeCode::create(ticket_type.id, code.id)
        .commit(connection)
        .unwrap();
    TicketTypeCode::create(ticket_type2.id, code.id)
        .commit(connection)
        .unwrap();
    TicketTypeCode::create(ticket_type3.id, code.id)
        .commit(connection)
        .unwrap();

    let mut display_code = code.for_display(connection).unwrap();
    assert_eq!(
        display_code.display_code.ticket_type_ids.sort(),
        vec![ticket_type.id, ticket_type2.id, ticket_type3.id].sort()
    );

    TicketTypeCode::destroy_multiple(code.id, vec![ticket_type.id, ticket_type3.id], connection).unwrap();

    let display_code = code.for_display(connection).unwrap();
    assert_eq!(display_code.display_code.ticket_type_ids, vec![ticket_type2.id]);
}

#[test]
fn create_with_validation_errors() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let code = project.create_code().finish();
    let event = project.create_event().with_ticket_pricing().finish();
    let ticket_type = &event.ticket_types(true, None, connection).unwrap()[0];

    let result = TicketTypeCode::create(ticket_type.id, code.id).commit(connection);

    match result {
        Ok(_) => {
            panic!("Expected validation error");
        }
        Err(error) => match &error.error_code {
            ValidationError { errors } => {
                assert!(errors.contains_key("ticket_type_id"));
                assert_eq!(errors["ticket_type_id"].len(), 1);
                assert_eq!(errors["ticket_type_id"][0].code, "invalid");
                assert_eq!(
                    &errors["ticket_type_id"][0].message.clone().unwrap().into_owned(),
                    "Ticket type not valid for code as it does not belong to same event"
                );
            }
            _ => panic!("Expected validation error"),
        },
    }
}
