use chrono::NaiveDateTime;
use diesel;
use diesel::prelude::*;
use models::*;
use schema::announcement_engagements;
use utils::errors::*;
use uuid::Uuid;

#[derive(Deserialize, Identifiable, Queryable, Serialize, Debug, PartialEq, Clone)]
pub struct AnnouncementEngagement {
    pub id: Uuid,
    pub user_id: Uuid,
    pub announcement_id: Uuid,
    pub action: AnnouncementEngagementAction,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Insertable, Serialize, Deserialize, PartialEq, Debug)]
#[table_name = "announcement_engagements"]
pub struct NewAnnouncementEngagement {
    pub user_id: Uuid,
    pub announcement_id: Uuid,
    pub action: AnnouncementEngagementAction,
}

impl AnnouncementEngagement {
    pub fn create(
        user_id: Uuid,
        announcement_id: Uuid,
        action: AnnouncementEngagementAction,
    ) -> NewAnnouncementEngagement {
        NewAnnouncementEngagement {
            user_id,
            announcement_id,
            action,
        }
    }

    pub fn find(id: Uuid, connection: &PgConnection) -> Result<AnnouncementEngagement, DatabaseError> {
        announcement_engagements::table
            .filter(announcement_engagements::id.eq(id))
            .get_result(connection)
            .to_db_error(ErrorCode::QueryError, "Unable to load announcement engagement")
    }

    pub fn find_by_announcement_id_user_id(
        announcement_id: Uuid,
        user_id: Uuid,
        connection: &PgConnection,
    ) -> Result<AnnouncementEngagement, DatabaseError> {
        announcement_engagements::table
            .filter(announcement_engagements::announcement_id.eq(announcement_id))
            .filter(announcement_engagements::user_id.eq(user_id))
            .get_result(connection)
            .to_db_error(ErrorCode::QueryError, "Unable to load announcement engagement")
    }
}

impl NewAnnouncementEngagement {
    pub fn commit(&self, connection: &PgConnection) -> Result<AnnouncementEngagement, DatabaseError> {
        diesel::insert_into(announcement_engagements::table)
            .values(self)
            .get_result(connection)
            .to_db_error(ErrorCode::InsertError, "Could not create new announcement engagement")
    }
}
