use db::dev::TestProject;
use db::prelude::*;
use db::utils::errors::ErrorCode::ValidationError;
use diesel;
use diesel::query_dsl::RunQueryDsl;
use diesel::sql_types;
use std::iter::repeat;

#[test]
fn create_commit() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let user = project.create_user().finish();
    let message = "Announcement message".to_string();
    let announcement = Announcement::create(None, message.clone())
        .commit(Some(user.id), connection)
        .unwrap();

    assert_eq!(announcement.message, message);
    assert!(announcement.organization_id.is_none());

    let domain_events = DomainEvent::find(
        Tables::Announcements,
        Some(announcement.id),
        Some(DomainEventTypes::AnnouncementCreated),
        connection,
    )
    .unwrap();
    assert_eq!(1, domain_events.len());
    assert_eq!(domain_events[0].user_id, Some(user.id));
}

#[test]
fn create_with_validation_errors() {
    let project = TestProject::new();
    let connection = project.get_connection();

    // Hits validation error when attempting to add over 190 characters
    let message = repeat("X").take(191).collect::<String>();
    let result = Announcement::create(None, message.clone()).commit(None, connection);
    match result {
        Ok(_) => {
            panic!("Expected validation error");
        }
        Err(error) => match &error.error_code {
            ValidationError { errors } => {
                assert!(errors.contains_key("message"));
                assert_eq!(errors["message"].len(), 1);
                assert_eq!(errors["message"][0].code, "length");
            }
            _ => panic!("Expected validation error"),
        },
    }

    // Using under 190 or less works
    let message = repeat("X").take(190).collect::<String>();
    assert!(Announcement::create(None, message.clone())
        .commit(None, connection)
        .is_ok());
}

#[test]
fn update() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let announcement = project.create_announcement().finish();
    let new_message = "Something new".to_string();
    let updated_announcement = announcement
        .update(
            AnnouncementEditableAttributes {
                message: Some(new_message.clone()),
                ..Default::default()
            },
            connection,
        )
        .unwrap();
    assert_eq!(new_message, updated_announcement.message);
}

#[test]
fn find() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let announcement = project.create_announcement().finish();
    let found_announcement = Announcement::find(announcement.id, false, connection).unwrap();
    assert_eq!(found_announcement, announcement);
}

#[test]
fn find_active_for_organization_user() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let organization = project.create_organization().finish();
    let organization2 = project.create_organization().finish();
    let user = project.create_user().finish();
    let user2 = project.create_user().finish();

    let announcement = project.create_announcement().finish();
    diesel::sql_query(
        r#"
        UPDATE announcements
        SET created_at = $1
        WHERE id = $2;
        "#,
    )
    .bind::<sql_types::Timestamp, _>(dates::now().add_hours(-1).finish())
    .bind::<sql_types::Uuid, _>(announcement.id)
    .execute(connection)
    .unwrap();
    let announcement = Announcement::find(announcement.id, false, connection).unwrap();
    let announcement2 = project.create_announcement().with_organization(&organization).finish();
    let announcement3 = project.create_announcement().with_organization(&organization2).finish();

    let found_announcements =
        Announcement::find_active_for_organization_user(organization.id, user.id, connection).unwrap();
    assert_eq!(found_announcements, vec![announcement.clone(), announcement2.clone()]);

    let found_announcements =
        Announcement::find_active_for_organization_user(organization2.id, user.id, connection).unwrap();
    assert_eq!(found_announcements, vec![announcement.clone(), announcement3.clone()]);

    // Announcement 3 is deleted
    announcement3.delete(None, connection).unwrap();

    let found_announcements =
        Announcement::find_active_for_organization_user(organization.id, user.id, connection).unwrap();
    assert_eq!(found_announcements, vec![announcement.clone(), announcement2.clone()]);

    let found_announcements =
        Announcement::find_active_for_organization_user(organization2.id, user.id, connection).unwrap();
    assert_eq!(found_announcements, vec![announcement.clone()]);

    // User 1 engages with announcement, user 2 engages with announcement 2
    project
        .create_announcement_engagement()
        .with_user(&user)
        .with_announcement(&announcement)
        .finish();
    project
        .create_announcement_engagement()
        .with_user(&user2)
        .with_announcement(&announcement2)
        .finish();

    let found_announcements =
        Announcement::find_active_for_organization_user(organization.id, user.id, connection).unwrap();
    assert_eq!(found_announcements, vec![announcement2.clone()]);
    let found_announcements =
        Announcement::find_active_for_organization_user(organization2.id, user.id, connection).unwrap();
    assert_eq!(found_announcements, vec![]);

    let found_announcements =
        Announcement::find_active_for_organization_user(organization.id, user2.id, connection).unwrap();
    assert_eq!(found_announcements, vec![announcement.clone()]);
    let found_announcements =
        Announcement::find_active_for_organization_user(organization2.id, user2.id, connection).unwrap();
    assert_eq!(found_announcements, vec![announcement.clone()]);
}

#[test]
fn all() {
    let project = TestProject::new();
    let connection = project.get_connection();
    assert!(Announcement::all(0, 100, connection).unwrap().is_empty());

    let announcement = project.create_announcement().finish();
    diesel::sql_query(
        r#"
        UPDATE announcements
        SET created_at = $1
        WHERE id = $2;
        "#,
    )
    .bind::<sql_types::Timestamp, _>(dates::now().add_hours(-1).finish())
    .bind::<sql_types::Uuid, _>(announcement.id)
    .execute(connection)
    .unwrap();
    let announcement = Announcement::find(announcement.id, false, connection).unwrap();

    let announcement2 = project.create_announcement().finish();
    diesel::sql_query(
        r#"
        UPDATE announcements
        SET created_at = $1
        WHERE id = $2;
        "#,
    )
    .bind::<sql_types::Timestamp, _>(dates::now().add_minutes(-30).finish())
    .bind::<sql_types::Uuid, _>(announcement2.id)
    .execute(connection)
    .unwrap();
    let announcement2 = Announcement::find(announcement2.id, false, connection).unwrap();

    let announcement3 = project.create_announcement().finish();
    let found_announcements = Announcement::all(0, 100, connection).unwrap();
    assert_eq!(
        found_announcements.data,
        vec![announcement.clone(), announcement2.clone(), announcement3.clone()]
    );

    // delete announcement3
    announcement3.delete(None, connection).unwrap();
    let found_announcements = Announcement::all(0, 100, connection).unwrap();
    assert_eq!(
        found_announcements.data,
        vec![announcement.clone(), announcement2.clone()]
    );
}

#[test]
fn delete() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let user = project.create_user().finish();
    let announcement = project.create_announcement().finish();

    let domain_events = DomainEvent::find(
        Tables::Announcements,
        Some(announcement.id),
        Some(DomainEventTypes::AnnouncementDeleted),
        connection,
    )
    .unwrap();
    assert_eq!(0, domain_events.len());

    announcement.delete(Some(user.id), connection).unwrap();

    let announcement = Announcement::find(announcement.id, true, connection).unwrap();
    assert!(announcement.deleted_at.is_some());
    let domain_events = DomainEvent::find(
        Tables::Announcements,
        Some(announcement.id),
        Some(DomainEventTypes::AnnouncementDeleted),
        connection,
    )
    .unwrap();
    assert_eq!(1, domain_events.len());
    assert_eq!(domain_events[0].user_id, Some(user.id));
}
