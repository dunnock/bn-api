use diesel::prelude::*;
use models::*;
use uuid::Uuid;

pub struct AnnouncementBuilder<'a> {
    message: String,
    organization_id: Option<Uuid>,
    deleted: bool,
    connection: &'a PgConnection,
}

impl<'a> AnnouncementBuilder<'a> {
    pub fn new(connection: &'a PgConnection) -> Self {
        AnnouncementBuilder {
            message: "Announcement to all of studio".to_string(),
            organization_id: None,
            deleted: false,
            connection,
        }
    }

    pub fn deleted(mut self) -> Self {
        self.deleted = true;
        self
    }

    pub fn with_message(mut self, message: String) -> Self {
        self.message = message;
        self
    }

    pub fn with_organization(mut self, organization: &Organization) -> Self {
        self.organization_id = Some(organization.id.clone());
        self
    }

    pub fn finish(&self) -> Announcement {
        let announcement = Announcement::create(self.organization_id, self.message.clone())
            .commit(None, self.connection)
            .unwrap();
        if self.deleted {
            announcement.delete(None, self.connection).unwrap();
        }
        announcement
    }
}
