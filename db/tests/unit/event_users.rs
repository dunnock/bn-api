use db::dev::TestProject;
use db::models::*;

#[test]
fn find_all_by_event_id() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let user = project.create_user().finish();
    let user2 = project.create_user().finish();
    let event = project.create_event().finish();
    let event2 = project.create_event().finish();

    let event_user = EventUser::create(user.id, event.id, Roles::PromoterReadOnly)
        .commit(connection)
        .unwrap();
    let event_user2 = EventUser::create(user.id, event2.id, Roles::PromoterReadOnly)
        .commit(connection)
        .unwrap();
    let event_user3 = EventUser::create(user2.id, event.id, Roles::PromoterReadOnly)
        .commit(connection)
        .unwrap();
    let event_users = EventUser::find_all_by_event_id(event.id, connection).unwrap();
    assert_equiv!(event_users, vec![event_user, event_user3]);
    let event_users = EventUser::find_all_by_event_id(event2.id, connection).unwrap();
    assert_equiv!(event_users, vec![event_user2]);
}

#[test]
fn destroy_all() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let user = project.create_user().finish();
    let user2 = project.create_user().finish();
    let event = project.create_event().finish();
    let event2 = project.create_event().finish();

    EventUser::create(user.id, event.id, Roles::PromoterReadOnly)
        .commit(connection)
        .unwrap();
    EventUser::create(user.id, event2.id, Roles::PromoterReadOnly)
        .commit(connection)
        .unwrap();
    EventUser::create(user2.id, event.id, Roles::PromoterReadOnly)
        .commit(connection)
        .unwrap();
    assert!(EventUser::destroy_all(user.id, connection).is_ok());
    assert!(EventUser::find_by_event_id_user_id(event.id, user.id, connection).is_err());
    assert!(EventUser::find_by_event_id_user_id(event2.id, user.id, connection).is_err());
    assert!(EventUser::find_by_event_id_user_id(event.id, user2.id, connection).is_ok());
}

#[test]
fn create() {
    let project = TestProject::new();
    let user = project.create_user().finish();
    let event = project.create_event().finish();

    let event_user = EventUser::create(user.id, event.id, Roles::PromoterReadOnly)
        .commit(project.get_connection())
        .unwrap();

    assert_eq!(event_user.user_id, user.id);
    assert_eq!(event_user.event_id, event.id);
    assert_eq!(event_user.role, Roles::PromoterReadOnly);
}

#[test]
fn find_by_event_id_user_id() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let user = project.create_user().finish();
    let event = project.create_event().finish();

    let event_user = EventUser::create(user.id, event.id, Roles::PromoterReadOnly)
        .commit(project.get_connection())
        .unwrap();

    assert_eq!(
        event_user,
        EventUser::find_by_event_id_user_id(event.id, user.id, connection).unwrap()
    );
}

#[test]
fn update_or_create() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let user = project.create_user().finish();
    let event = project.create_event().finish();
    let event2 = project.create_event().finish();
    let event3 = project.create_event().finish();

    EventUser::create(user.id, event.id, Roles::Promoter)
        .commit(project.get_connection())
        .unwrap();
    EventUser::create(user.id, event2.id, Roles::Promoter)
        .commit(project.get_connection())
        .unwrap();

    // Add a new promoter (we'll have 3 after this) and update event_user2 to have readonly role
    EventUser::update_or_create(
        user.id,
        &vec![event2.id, event3.id],
        Roles::PromoterReadOnly,
        connection,
    )
    .unwrap();

    // Confirm promoters were added / updated correctly
    let event_user = EventUser::find_by_event_id_user_id(event.id, user.id, connection).unwrap();
    let event_user2 = EventUser::find_by_event_id_user_id(event2.id, user.id, connection).unwrap();
    let event_user3 = EventUser::find_by_event_id_user_id(event3.id, user.id, connection).unwrap();
    assert_eq!(event_user.role, Roles::Promoter);
    assert_eq!(event_user2.role, Roles::PromoterReadOnly);
    assert_eq!(event_user3.role, Roles::PromoterReadOnly);
}

#[test]
fn update() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let user = project.create_user().finish();
    let event = project.create_event().finish();

    let event_user = EventUser::create(user.id, event.id, Roles::Promoter)
        .commit(project.get_connection())
        .unwrap();

    let parameters = EventUserEditableAttributes {
        role: Some(Roles::PromoterReadOnly),
    };
    let event_user = event_user.update(&parameters, connection).unwrap();

    assert_eq!(event_user.role, Roles::PromoterReadOnly);
}

#[test]
fn destroy() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let user = project.create_user().finish();
    let event = project.create_event().finish();

    let event_user = EventUser::create(user.id, event.id, Roles::PromoterReadOnly)
        .commit(project.get_connection())
        .unwrap();
    assert!(event_user.destroy(connection).is_ok());
}
