use chrono::NaiveDateTime;
use diesel;
use diesel::prelude::*;
use schema::rarities;
use utils::errors::ConvertToDatabaseError;
use utils::errors::DatabaseError;
use utils::errors::ErrorCode;
use uuid::Uuid;

#[derive(Deserialize, Identifiable, Queryable, Debug, Serialize)]
#[table_name = "rarities"]
pub struct Rarity {
    pub id: Uuid,
    pub event_id: Option<Uuid>,
    pub name: String,
    pub rank: i32,
    pub color: Option<String>,
    created_at: NaiveDateTime,
    updated_at: NaiveDateTime,
}

#[derive(Insertable, Deserialize)]
#[table_name = "rarities"]
pub struct NewRarity {
    pub name: String,
    pub event_id: Option<Uuid>,
    pub rank: i32,
}

impl Rarity {}

impl NewRarity {
    pub fn commit(self, conn: &PgConnection) -> Result<Rarity, DatabaseError> {
        diesel::insert_into(rarities::table)
            .values(self)
            .get_result(conn)
            .to_db_error(ErrorCode::InsertError, "Could not create rarity")
    }
}
