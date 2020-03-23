use chrono::NaiveDateTime;
use diesel::dsl;
use diesel::prelude::*;
use models::MarketplaceAccountStatus;
use schema::*;
use utils::errors::ConvertToDatabaseError;
use utils::errors::{DatabaseError, ErrorCode};
use uuid::Uuid;

#[derive(Queryable, Identifiable)]
#[table_name = "marketplace_accounts"]
pub struct MarketplaceAccount {
    pub id: Uuid,
    pub user_id: Uuid,
    pub status: MarketplaceAccountStatus,
    pub marketplace_id: Option<String>,
    pub marketplace_user_id: String,
    pub marketplace_password: String,
    pub deleted_at: Option<NaiveDateTime>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

impl MarketplaceAccount {
    pub fn create(user_id: Uuid, marketplace_user_id: String, marketplace_password: String) -> NewMarketplaceAccount {
        NewMarketplaceAccount {
            user_id,
            status: MarketplaceAccountStatus::Pending,
            marketplace_user_id,
            marketplace_password,
        }
    }

    pub fn find_by_user_id(user_id: Uuid, conn: &PgConnection) -> Result<Vec<MarketplaceAccount>, DatabaseError> {
        marketplace_accounts::table
            .filter(marketplace_accounts::user_id.eq(user_id))
            .filter(marketplace_accounts::deleted_at.is_null())
            .get_results(conn)
            .to_db_error(ErrorCode::QueryError, "Could not find marketplace account for user")
    }

    pub fn update_marketplace_id(self, id: String, conn: &PgConnection) -> Result<MarketplaceAccount, DatabaseError> {
        diesel::update(&self)
            .set((
                marketplace_accounts::marketplace_id.eq(id),
                marketplace_accounts::status.eq(MarketplaceAccountStatus::Linked),
                marketplace_accounts::updated_at.eq(dsl::now),
            ))
            .get_result(conn)
            .to_db_error(ErrorCode::UpdateError, "Could not set marketplace account as linled")
    }
}

#[derive(Insertable)]
#[table_name = "marketplace_accounts"]
pub struct NewMarketplaceAccount {
    user_id: Uuid,
    status: MarketplaceAccountStatus,
    marketplace_user_id: String,
    marketplace_password: String,
}

impl NewMarketplaceAccount {
    pub fn commit(self, conn: &PgConnection) -> Result<MarketplaceAccount, DatabaseError> {
        diesel::insert_into(marketplace_accounts::table)
            .values((
                self,
                marketplace_accounts::created_at.eq(dsl::now),
                marketplace_accounts::updated_at.eq(dsl::now),
            ))
            .get_result(conn)
            .to_db_error(ErrorCode::InsertError, "Could not create marketplace account")
    }
}
