use chrono::NaiveDateTime;
use diesel;
use diesel::prelude::*;
use schema::loot_box_contents;
use utils::errors::ConvertToDatabaseError;
use utils::errors::DatabaseError;
use utils::errors::ErrorCode;
use uuid::Uuid;

#[derive(Deserialize, Identifiable, Queryable, Debug, Serialize)]
pub struct LootBoxContent {
    pub id: Uuid,
    pub ticket_type_id: Uuid,
    pub content_event_id: Uuid,
    pub min_rarity_id: Option<Uuid>,
    pub max_rarity_id: Option<Uuid>,
    pub content_ticket_type_id: Option<Uuid>,
    pub quantity_per_box: i32,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Deserialize, Insertable, Clone)]
#[table_name = "loot_box_contents"]
pub struct NewLootBoxContent {
    pub ticket_type_id: Uuid,
    pub content_event_id: Uuid,
    pub min_rarity_id: Option<Uuid>,
    pub max_rarity_id: Option<Uuid>,
    pub content_ticket_type_id: Option<Uuid>,
    pub quantity_per_box: i32,
}

impl NewLootBoxContent {
    pub fn commit(self, _current_user_id: Option<Uuid>, conn: &PgConnection) -> Result<LootBoxContent, DatabaseError> {
        diesel::insert_into(loot_box_contents::table)
            .values(self)
            .get_result(conn)
            .to_db_error(ErrorCode::InsertError, "Could not create loot box content")
        // TODO: Create domain event
    }
}
