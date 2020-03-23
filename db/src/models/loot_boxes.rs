use chrono::NaiveDateTime;
use diesel;
use diesel::expression::dsl;
use diesel::expression::dsl::sql;
use diesel::prelude::*;
use diesel::sql_types;
use schema::loot_boxes;
use utils::errors::ConvertToDatabaseError;
use utils::errors::DatabaseError;
use utils::errors::ErrorCode;
use uuid::Uuid;
use models::LootBoxInstance;


#[derive(Deserialize, Identifiable, Queryable, Debug, Serialize)]
#[table_name= "loot_boxes"]
pub struct LootBox {
    pub id: Uuid,
    pub promo_image_url: Option<String>,
    pub name: String    ,
    pub price_in_cents: i64,
    pub description: Option<String>,
    pub rank: i32,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime
}

#[derive(Insertable, Deserialize)]
#[table_name="loot_boxes"]
pub struct NewLootBox {
    pub name: String,
    pub promo_image_url: Option<String>,
    pub price_in_cents: i64
}


impl LootBox {
//    pub fn set_quantity(&self, user_id: Option<Uuid>, quantity: u32, conn: &PgConnection) -> Result<(), DatabaseError >{
//        let (count, _) = self.quantity(conn)?;
//        unimplemented!();
//    }

    pub fn quantity(&self, conn: &PgConnection) -> Result<(i64, i64), DatabaseError> {
        LootBoxInstance::count_for_loot_box(self.id, conn)
    }
}

impl NewLootBox {


    pub fn commit(self, conn: &PgConnection) -> Result<LootBox, DatabaseError> {
        diesel::insert_into(loot_boxes::table).values(self).get_result(conn).to_db_error(ErrorCode::InsertError,"Could not create loot box")
    }
}
