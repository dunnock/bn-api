use db::dev::TestProject;
use db::prelude::*;

#[test]
fn create_commit() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let user = project.create_user().finish();
    let announcement = project.create_announcement().finish();
    let announcement_engagement =
        AnnouncementEngagement::create(user.id, announcement.id, AnnouncementEngagementAction::Dismiss)
            .commit(connection)
            .unwrap();

    assert_eq!(announcement_engagement.announcement_id, announcement.id);
    assert_eq!(announcement_engagement.user_id, user.id);
}

#[test]
fn find() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let announcement_engagement = project.create_announcement_engagement().finish();
    let found_announcement_engagement = AnnouncementEngagement::find(announcement_engagement.id, connection).unwrap();
    assert_eq!(found_announcement_engagement, announcement_engagement);
}

#[test]
fn find_by_announcement_id_user_id() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let user = project.create_user().finish();
    let announcement = project.create_announcement().finish();
    let announcement_engagement = project
        .create_announcement_engagement()
        .with_user(&user)
        .with_announcement(&announcement)
        .finish();
    let found_announcement_engagement =
        AnnouncementEngagement::find_by_announcement_id_user_id(announcement.id, user.id, connection).unwrap();
    assert_eq!(found_announcement_engagement, announcement_engagement);
}
