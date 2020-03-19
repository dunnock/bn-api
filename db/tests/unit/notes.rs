use db::dev::TestProject;
use db::models::*;
use db::utils::dates;
use diesel;
use diesel::sql_types;
use diesel::RunQueryDsl;
use validator::Validate;

#[test]
fn create() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let user = project.create_user().finish();
    let order = project.create_order().is_paid().finish();
    let note_text = "Note goes here".to_string();
    assert_eq!(
        0,
        DomainEvent::find(
            Tables::Orders,
            Some(order.id),
            Some(DomainEventTypes::NoteCreated),
            connection,
        )
        .unwrap()
        .len()
    );

    let note = Note::create(Tables::Orders, order.id, user.id, note_text.clone())
        .commit(connection)
        .unwrap();
    assert_eq!(note.note, note_text);
    assert_eq!(note.created_by, user.id);
    assert_eq!(note.main_id, order.id);
    assert_eq!(note.main_table, Tables::Orders);
    assert_eq!(
        1,
        DomainEvent::find(
            Tables::Orders,
            Some(order.id),
            Some(DomainEventTypes::NoteCreated),
            connection,
        )
        .unwrap()
        .len()
    );
}

#[test]
fn find() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let note = project.create_note().finish();
    assert_eq!(note, Note::find(note.id, connection).unwrap());
}

#[test]
fn destroy() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let note = project.create_note().finish();
    let user = project.create_user().finish();
    assert_eq!(
        0,
        DomainEvent::find(
            Tables::Orders,
            Some(note.main_id),
            Some(DomainEventTypes::NoteDeleted),
            connection,
        )
        .unwrap()
        .len()
    );

    // Soft deleted note returns error as it's not found
    assert!(note.destroy(user.id, connection).is_ok());

    // Reload note
    let note = Note::find(note.id, connection).unwrap();
    assert!(note.deleted_at.is_some());
    assert_eq!(note.deleted_by, Some(user.id));
    assert_eq!(
        1,
        DomainEvent::find(
            Tables::Orders,
            Some(note.main_id),
            Some(DomainEventTypes::NoteDeleted),
            connection,
        )
        .unwrap()
        .len()
    );
}

#[test]
fn new_note_validate() {
    let project = TestProject::new();
    let user = project.create_user().finish();
    let order = project.create_order().is_paid().finish();
    let note = Note::create(Tables::Orders, order.id, user.id, "".to_string());

    let result = note.validate();
    assert!(result.is_err());
    let errors = result.unwrap_err().field_errors();

    assert!(errors.contains_key("note"));
    assert_eq!(errors["note"].len(), 1);
    assert_eq!(errors["note"][0].code, "length");
    assert_eq!(
        &errors["note"][0].message.clone().unwrap().into_owned(),
        "Note cannot be blank"
    );
}

#[test]
fn find_for_table() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let user = project.create_user().finish();
    let order = project.create_order().is_paid().finish();
    let note = project.create_note().for_order(&order).finish();
    let order2 = project.create_order().is_paid().finish();
    let note2 = project.create_note().for_order(&order2).finish();
    let note3 = project.create_note().for_order(&order2).finish();
    // order by created_at desc so note3 with an older created_at will appear last
    diesel::sql_query(
        r#"
        UPDATE notes
        SET created_at = $1
        WHERE id = $2;
        "#,
    )
    .bind::<sql_types::Timestamp, _>(dates::now().add_hours(-2).finish())
    .bind::<sql_types::Uuid, _>(note3.id)
    .execute(connection)
    .unwrap();
    let note3 = Note::find(note3.id, connection).unwrap();

    assert_eq!(
        vec![note.clone()],
        Note::find_for_table(Tables::Orders, order.id, true, 0, 100, connection)
            .unwrap()
            .data
    );
    assert_eq!(
        vec![note2.clone(), note3.clone()],
        Note::find_for_table(Tables::Orders, order2.id, true, 0, 100, connection)
            .unwrap()
            .data
    );

    // Soft deleted notes do not appear in result set when set to filter deleted
    assert!(note3.destroy(user.id, connection).is_ok());
    let note3 = Note::find(note3.id, connection).unwrap();
    assert_eq!(
        vec![note.clone()],
        Note::find_for_table(Tables::Orders, order.id, true, 0, 100, connection)
            .unwrap()
            .data
    );
    assert_eq!(
        vec![note2.clone()],
        Note::find_for_table(Tables::Orders, order2.id, true, 0, 100, connection)
            .unwrap()
            .data
    );

    // Setting filter deleted to false shows the entire result set including deleted notes
    assert_eq!(
        vec![note],
        Note::find_for_table(Tables::Orders, order.id, false, 0, 100, connection)
            .unwrap()
            .data
    );
    assert_eq!(
        vec![note2.clone(), note3.clone()],
        Note::find_for_table(Tables::Orders, order2.id, false, 0, 100, connection)
            .unwrap()
            .data
    );

    // Pagination support
    let pagination_result = Note::find_for_table(Tables::Orders, order2.id, false, 0, 1, connection).unwrap();
    assert_eq!(vec![note2], pagination_result.data);
    assert_eq!(2, pagination_result.paging.total);

    assert_eq!(
        vec![note3],
        Note::find_for_table(Tables::Orders, order2.id, false, 1, 1, connection)
            .unwrap()
            .data
    );
}
