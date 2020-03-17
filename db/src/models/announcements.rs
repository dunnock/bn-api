use chrono::NaiveDateTime;
use diesel;
use diesel::expression::dsl;
use diesel::prelude::*;
use models::*;
use schema::{announcement_engagements, announcements};
use utils::errors::*;
use utils::pagination::Paginate;
use uuid::Uuid;
use validator::Validate;

#[derive(Deserialize, Identifiable, Queryable, Serialize, Debug, PartialEq, Clone)]
pub struct Announcement {
    pub id: Uuid,
    pub message: String,
    pub organization_id: Option<Uuid>,
    pub deleted_at: Option<NaiveDateTime>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Default, Insertable, Serialize, Deserialize, PartialEq, Debug, Validate)]
#[table_name = "announcements"]
pub struct NewAnnouncement {
    pub organization_id: Option<Uuid>,
    #[validate(length(max = 190))]
    pub message: String,
}

#[derive(AsChangeset, Default, Deserialize, Debug, Validate)]
#[table_name = "announcements"]
pub struct AnnouncementEditableAttributes {
    pub organization_id: Option<Option<Uuid>>,
    #[serde(default, deserialize_with = "deserialize_unless_blank")]
    #[validate(length(max = 190))]
    pub message: Option<String>,
}

impl Announcement {
    pub fn create(organization_id: Option<Uuid>, message: String) -> NewAnnouncement {
        NewAnnouncement {
            organization_id,
            message,
        }
    }

    pub fn update(
        &self,
        attributes: AnnouncementEditableAttributes,
        conn: &PgConnection,
    ) -> Result<Announcement, DatabaseError> {
        attributes.validate()?;
        diesel::update(self)
            .set((attributes, announcements::updated_at.eq(dsl::now)))
            .get_result(conn)
            .to_db_error(ErrorCode::UpdateError, "Could not update announcement")
    }

    pub fn find(id: Uuid, include_deleted: bool, connection: &PgConnection) -> Result<Announcement, DatabaseError> {
        let mut query = announcements::table.filter(announcements::id.eq(id)).into_boxed();

        if !include_deleted {
            query = query.filter(announcements::deleted_at.is_null());
        }

        query
            .get_result(connection)
            .to_db_error(ErrorCode::QueryError, "Unable to load announcement")
    }

    pub fn find_active_for_organization_user(
        organization_id: Uuid,
        user_id: Uuid,
        connection: &PgConnection,
    ) -> Result<Vec<Announcement>, DatabaseError> {
        announcements::table
            .left_join(announcement_engagements::table.on(announcement_engagements::user_id.eq(user_id).and(announcements::id.eq(announcement_engagements::announcement_id))))
            .filter(announcements::organization_id.eq(Some(organization_id)).or(announcements::organization_id.is_null()))
            .filter(announcements::deleted_at.is_null())
            .filter(announcement_engagements::id.is_null()) // Any engagement removes announcement from queue
            .select(announcements::all_columns)
            .get_results(connection)
            .to_db_error(ErrorCode::QueryError, "Unable to load announcements")
    }

    pub fn all(page: i64, limit: i64, conn: &PgConnection) -> Result<Payload<Announcement>, DatabaseError> {
        let (announcements, total) = announcements::table
            .filter(announcements::deleted_at.is_null())
            .then_order_by(announcements::created_at.asc())
            .paginate(page as i64)
            .per_page(limit as i64)
            .load_and_count_pages(conn)
            .to_db_error(ErrorCode::QueryError, "Unable to load all announcements")?;

        Ok(Payload::from_data(
            announcements,
            page as u32,
            limit as u32,
            Some(total as u64),
        ))
    }

    pub fn delete(&self, current_user_id: Option<Uuid>, conn: &PgConnection) -> Result<(), DatabaseError> {
        DomainEvent::create(
            DomainEventTypes::AnnouncementDeleted,
            format!("Announcement '{}' deleted", &self.message),
            Tables::Announcements,
            Some(self.id),
            current_user_id,
            Some(json!(&self)),
        )
        .commit(conn)?;

        diesel::update(self)
            .set((
                announcements::deleted_at.eq(dsl::now),
                announcements::updated_at.eq(dsl::now),
            ))
            .execute(conn)
            .to_db_error(ErrorCode::UpdateError, "Could not delete announcement")?;

        Ok(())
    }
}

impl NewAnnouncement {
    pub fn commit(&self, current_user_id: Option<Uuid>, conn: &PgConnection) -> Result<Announcement, DatabaseError> {
        self.validate()?;
        let announcement: Announcement = diesel::insert_into(announcements::table)
            .values(self)
            .get_result(conn)
            .to_db_error(ErrorCode::InsertError, "Could not create new announcement")?;

        DomainEvent::create(
            DomainEventTypes::AnnouncementCreated,
            format!("Announcement '{}' created", &self.message),
            Tables::Announcements,
            Some(announcement.id),
            current_user_id,
            Some(json!(&announcement)),
        )
        .commit(conn)?;

        Ok(announcement)
    }
}
