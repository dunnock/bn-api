use chrono::NaiveDateTime;
use diesel;
use diesel::prelude::*;
use models::*;
use schema::{organization_venues, organizations, venues};
use utils::errors::DatabaseError;
use utils::errors::ErrorCode;
use utils::errors::*;
use utils::pagination::Paginate;
use uuid::Uuid;

#[derive(AsChangeset, Associations, Debug, Identifiable, PartialEq, Queryable, Serialize, Deserialize)]
#[belongs_to(Venue)]
#[belongs_to(Organization)]
#[table_name = "organization_venues"]
pub struct OrganizationVenue {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub venue_id: Uuid,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Insertable, Serialize, Deserialize, PartialEq, Debug)]
#[table_name = "organization_venues"]
pub struct NewOrganizationVenue {
    pub organization_id: Uuid,
    pub venue_id: Uuid,
}

impl NewOrganizationVenue {
    pub fn commit(self, conn: &PgConnection) -> Result<OrganizationVenue, DatabaseError> {
        diesel::insert_into(organization_venues::table)
            .values(&self)
            .get_result(conn)
            .to_db_error(
                ErrorCode::InsertError,
                "Could not create new organization venue table row",
            )
    }
}

impl OrganizationVenue {
    pub fn create(organization_id: Uuid, venue_id: Uuid) -> NewOrganizationVenue {
        NewOrganizationVenue {
            organization_id,
            venue_id,
        }
    }

    pub fn destroy(&self, conn: &PgConnection) -> Result<usize, DatabaseError> {
        if OrganizationVenue::find_by_venue(self.venue_id, None, None, conn)?
            .paging
            .total
            == 1
        {
            return DatabaseError::business_process_error(
                "Unable to remove organization venue link, at least one organization must be associated with venue",
            );
        }

        diesel::delete(self)
            .execute(conn)
            .to_db_error(ErrorCode::DeleteError, "Failed to delete organization venue")
    }

    pub fn find(id: Uuid, conn: &PgConnection) -> Result<OrganizationVenue, DatabaseError> {
        organization_venues::table
            .filter(organization_venues::id.eq(id))
            .get_result(conn)
            .to_db_error(ErrorCode::QueryError, "Could not load organizations")
    }

    pub fn find_by_venue(
        venue_id: Uuid,
        page: Option<u32>,
        limit: Option<u32>,
        conn: &PgConnection,
    ) -> Result<Payload<OrganizationVenue>, DatabaseError> {
        let limit = limit.unwrap_or(100);
        let page = page.unwrap_or(0);
        let (organization_venues, record_count): (Vec<OrganizationVenue>, i64) = organization_venues::table
            .inner_join(organizations::table.on(organizations::id.eq(organization_venues::organization_id)))
            .filter(organization_venues::venue_id.eq(venue_id))
            .order_by(organizations::name)
            .select(organization_venues::all_columns)
            .paginate(page as i64)
            .per_page(limit as i64)
            .load_and_count_pages(conn)
            .to_db_error(ErrorCode::QueryError, "Could not load organizations")?;

        let payload = Payload::from_data(organization_venues, page, limit, Some(record_count as u64));
        Ok(payload)
    }

    pub fn find_by_organization(
        organization_id: Uuid,
        page: Option<u32>,
        limit: Option<u32>,
        conn: &PgConnection,
    ) -> Result<Payload<OrganizationVenue>, DatabaseError> {
        let limit = limit.unwrap_or(100);
        let page = page.unwrap_or(0);
        let (organization_venues, record_count): (Vec<OrganizationVenue>, i64) = organization_venues::table
            .inner_join(venues::table.on(venues::id.eq(organization_venues::venue_id)))
            .filter(organization_venues::organization_id.eq(organization_id))
            .order_by(venues::name)
            .select(organization_venues::all_columns)
            .paginate(page as i64)
            .per_page(limit as i64)
            .load_and_count_pages(conn)
            .to_db_error(ErrorCode::QueryError, "Could not load organizations")?;

        let payload = Payload::from_data(organization_venues, page, limit, Some(record_count as u64));
        Ok(payload)
    }

    pub fn find_organizations_by_venue(
        venue_id: Uuid,
        conn: &PgConnection,
    ) -> Result<Vec<Organization>, DatabaseError> {
        organization_venues::table
            .inner_join(organizations::table.on(organizations::id.eq(organization_venues::organization_id)))
            .filter(organization_venues::venue_id.eq(venue_id))
            .order_by(organizations::name)
            .select(organizations::all_columns)
            .distinct()
            .load(conn)
            .to_db_error(ErrorCode::QueryError, "Could not load organizations")
    }

    pub fn find_venues_by_organization(
        organization_id: Uuid,
        conn: &PgConnection,
    ) -> Result<Vec<Venue>, DatabaseError> {
        organization_venues::table
            .inner_join(venues::table.on(venues::id.eq(organization_venues::venue_id)))
            .filter(organization_venues::organization_id.eq(organization_id))
            .order_by(venues::name)
            .select(venues::all_columns)
            .distinct()
            .load(conn)
            .to_db_error(ErrorCode::QueryError, "Could not load venues")
    }
}
