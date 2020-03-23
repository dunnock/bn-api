use chrono::NaiveDateTime;
use diesel::dsl;
use diesel::prelude::*;
use prelude::*;
use schema::*;
use utils::errors::ErrorCode;
use uuid::Uuid;

#[derive(Queryable, Identifiable)]
#[table_name = "listings"]
pub struct Listing {
    pub id: Uuid,
    pub title: String,
    pub user_id: Uuid,
    pub marketplace_id: Option<String>,
    pub asking_price_in_cents: i64,
    pub status: ListingStatus,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub deleted_at: Option<NaiveDateTime>,
}

impl Listing {
    pub fn create(title: String, user_id: Uuid, asking_price_in_cents: i64) -> NewListing {
        NewListing {
            title,
            user_id,
            asking_price_in_cents,
        }
    }

    pub fn find(id: Uuid, conn: &PgConnection) -> Result<Listing, DatabaseError> {
        listings::table
            .filter(listings::id.eq(id))
            .get_result(conn)
            .to_db_error(ErrorCode::QueryError, "Could not find listing")
    }

    pub fn set_published(self, marketplace_id: String, conn: &PgConnection) -> Result<Listing, DatabaseError> {
        diesel::update(&self)
            .set((
                listings::marketplace_id.eq(marketplace_id),
                listings::status.eq(ListingStatus::Published),
                listings::updated_at.eq(dsl::now),
            ))
            .get_result(conn)
            .to_db_error(ErrorCode::UpdateError, "Could not publish listing")
    }
}

#[derive(Insertable)]
#[table_name = "listings"]
pub struct NewListing {
    title: String,
    asking_price_in_cents: i64,
    user_id: Uuid,
}

impl NewListing {
    pub fn commit(self, conn: &PgConnection) -> Result<Listing, DatabaseError> {
        diesel::insert_into(listings::table)
            .values((
                &self,
                listings::status.eq(ListingStatus::Pending),
                listings::created_at.eq(dsl::now),
                listings::updated_at.eq(dsl::now),
            ))
            .get_result(conn)
            .to_db_error(ErrorCode::InsertError, "Could not create listing")
    }
}
