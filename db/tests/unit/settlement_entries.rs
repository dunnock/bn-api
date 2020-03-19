use chrono::prelude::*;
use db::dev::TestProject;
use db::models::*;
use db::utils::dates;

#[test]
fn find_for_settlement_by_event() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let organization = project.create_organization().finish();
    let event = project
        .create_event()
        .with_tickets()
        .with_ticket_type_count(2)
        .with_organization(&organization)
        .with_event_start(dates::now().add_days(-2).finish())
        .finish();
    let event2 = project
        .create_event()
        .with_organization(&organization)
        .with_event_start(dates::now().add_days(-1).finish())
        .finish();
    let mut ticket_types = event.ticket_types(false, None, connection).unwrap();
    let ticket_type = ticket_types.remove(0);
    let ticket_type2 = ticket_types.remove(0);
    let settlement = Settlement::create(
        organization.id,
        NaiveDate::from_ymd(2016, 7, 8).and_hms(4, 10, 11),
        NaiveDate::from_ymd(2020, 7, 8).and_hms(4, 10, 11),
        SettlementStatus::PendingSettlement,
        Some("test comment".to_string()),
        true,
    )
    .commit(None, connection)
    .unwrap();
    let settlement_entry = project
        .create_settlement_entry()
        .with_face_value_in_cents(100)
        .with_event(&event)
        .with_ticket_type_id(ticket_type.id)
        .with_settlement(&settlement)
        .finish();
    let settlement_entry2 = project
        .create_settlement_entry()
        .with_face_value_in_cents(80)
        .with_event(&event)
        .with_ticket_type_id(ticket_type2.id)
        .with_settlement(&settlement)
        .finish();
    let settlement_entry3 = project
        .create_settlement_entry()
        .with_face_value_in_cents(120)
        .with_event(&event)
        .with_settlement(&settlement)
        .finish();
    let settlement_entry4 = project
        .create_settlement_entry()
        .with_face_value_in_cents(60)
        .with_event(&event2)
        .with_settlement(&settlement)
        .finish();

    // Ticket type adjusts order
    ticket_type.clone().update_rank_only(0, connection).unwrap();
    ticket_type2.clone().update_rank_only(1, connection).unwrap();
    let mut results = SettlementEntry::find_for_settlement_by_event(&settlement, connection).unwrap();
    assert_eq!(results.len(), 2);
    let grouped_settlement_entry = results.pop().unwrap();
    assert_eq!(grouped_settlement_entry.event, event2.for_display(connection).unwrap());
    assert_eq!(grouped_settlement_entry.entries.len(), 1);
    assert_eq!(grouped_settlement_entry.entries[0].id, settlement_entry4.id);

    let grouped_settlement_entry = results.pop().unwrap();
    assert_eq!(grouped_settlement_entry.event, event.for_display(connection).unwrap());
    assert_eq!(grouped_settlement_entry.entries.len(), 3);
    assert_eq!(grouped_settlement_entry.entries[0].id, settlement_entry.id);
    assert_eq!(grouped_settlement_entry.entries[1].id, settlement_entry2.id);
    assert_eq!(grouped_settlement_entry.entries[2].id, settlement_entry3.id);

    // Adjust order in reverse for event
    ticket_type.update_rank_only(1, connection).unwrap();
    ticket_type2.update_rank_only(0, connection).unwrap();
    let mut results = SettlementEntry::find_for_settlement_by_event(&settlement, connection).unwrap();
    assert_eq!(results.len(), 2);
    let grouped_settlement_entry = results.pop().unwrap();
    assert_eq!(grouped_settlement_entry.entries.len(), 1);
    assert_eq!(grouped_settlement_entry.entries[0].id, settlement_entry4.id);

    let grouped_settlement_entry = results.pop().unwrap();
    assert_eq!(grouped_settlement_entry.entries.len(), 3);
    assert_eq!(grouped_settlement_entry.entries[0].id, settlement_entry2.id);
    assert_eq!(grouped_settlement_entry.entries[1].id, settlement_entry.id);
    assert_eq!(grouped_settlement_entry.entries[2].id, settlement_entry3.id);
}

#[test]
fn create() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let venue = project.create_venue().finish();
    let user = project.create_user().finish();
    let organization = project
        .create_organization()
        .with_member(&user, Roles::OrgOwner)
        .finish();
    let event = project
        .create_event()
        .with_name("NewEvent".into())
        .with_organization(&organization)
        .with_venue(&venue)
        .finish();
    let settlement = Settlement::create(
        organization.id,
        NaiveDate::from_ymd(2016, 7, 8).and_hms(4, 10, 11),
        NaiveDate::from_ymd(2020, 7, 8).and_hms(4, 10, 11),
        SettlementStatus::PendingSettlement,
        Some("test comment".to_string()),
        true,
    )
    .commit(None, connection)
    .unwrap();

    let settlement_entry = project
        .create_settlement_entry()
        .with_face_value_in_cents(100)
        .with_event(&event)
        .with_settlement(&settlement)
        .finish();

    assert_eq!(settlement_entry.event_id, event.id);
    assert_eq!(settlement_entry.ticket_type_id, None);
    assert_eq!(settlement_entry.face_value_in_cents, 100);
}
