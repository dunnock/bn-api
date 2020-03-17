use diesel::prelude::*;
use models::*;
use test::builders::*;
use uuid::Uuid;

pub struct AnnouncementEngagementBuilder<'a> {
    user_id: Option<Uuid>,
    announcement_id: Option<Uuid>,
    action: AnnouncementEngagementAction,
    connection: &'a PgConnection,
}

impl<'a> AnnouncementEngagementBuilder<'a> {
    pub fn new(connection: &'a PgConnection) -> Self {
        AnnouncementEngagementBuilder {
            user_id: None,
            announcement_id: None,
            action: AnnouncementEngagementAction::Dismiss,
            connection,
        }
    }

    pub fn with_user(mut self, user: &User) -> Self {
        self.user_id = Some(user.id.clone());
        self
    }

    pub fn with_announcement(mut self, announcement: &Announcement) -> Self {
        self.announcement_id = Some(announcement.id.clone());
        self
    }

    pub fn finish(&self) -> AnnouncementEngagement {
        let user_id = self
            .user_id
            .or_else(|| Some(UserBuilder::new(self.connection).finish().id))
            .unwrap();
        let announcement_id = self
            .announcement_id
            .or_else(|| Some(AnnouncementBuilder::new(self.connection).finish().id))
            .unwrap();
        AnnouncementEngagement::create(user_id, announcement_id, self.action)
            .commit(self.connection)
            .unwrap()
    }
}
