use chrono::NaiveDateTime;
use diesel;
use diesel::expression::dsl;
use diesel::expression::dsl::sql;
use diesel::prelude::*;
use diesel::sql_types;
use schema::loot_box_instances;
use utils::errors::*;
use uuid::Uuid;
use models::{LootBoxContent, TicketInstance};

#[derive(Deserialize, Identifiable, Queryable, Debug, Serialize)]
pub struct LootBoxInstance {
    pub id : Uuid,
    pub loot_box_id: Uuid,
    pub order_item_id: Option<Uuid>,
    pub wallet_id: Uuid,
    pub reserved_until: Option<NaiveDateTime>,
    pub status: String,
    pub opened_at: Option<NaiveDateTime>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime

}

#[derive(Insertable)]
#[table_name = "loot_box_instances"]
pub struct NewLootBoxInstance {
    pub loot_box_id: Uuid,
    pub wallet_id: Uuid,
    pub status: String
}

impl LootBoxInstance{
    pub fn count_for_loot_box(loot_box_id: Uuid, conn: &PgConnection) -> Result<(i64, i64), DatabaseError > {

        #[derive(Queryable)]
        struct R {
            count: Option<i64>,
            available_count: Option<i64>,
        };

        let result = loot_box_instances::table.filter(loot_box_instances::loot_box_id.eq(loot_box_id)).select((
            sql::<sql_types::Nullable<sql_types::BigInt>>("COUNT(DISTINCT loot_box_instances.id)"),
            sql::<sql_types::Nullable<sql_types::BigInt>>(
                "SUM(CASE WHEN loot_box_instances.status IN ('Available', 'Reserved') THEN 1 ELSE 0 END)",
            ),
        )).first::<R>(conn).to_db_error(ErrorCode::QueryError, "Could not retrieve the number of loot boxes").optional()?;

        match result {
            Some(r) => Ok((
                r.count.unwrap_or(0),
                r.available_count.unwrap_or(0),
            )),
            None => Ok((0, 0)),
        }
    }

    pub fn create_multiple(current_user_id: Option<Uuid>, loot_box_id: Uuid, quantity: i64, loot_box_content: &LootBoxContent, wallet_id : Uuid, conn: &PgConnection) -> Result<(), DatabaseError> {

//        let mut new_rows = vec![];
        for x in 0..quantity {
            let new_row = NewLootBoxInstance{
                loot_box_id,
                wallet_id,
                status: "Available".to_string()
            };

            let instance :LootBoxInstance= diesel::insert_into(loot_box_instances::table)
                .values(&new_row)
                .get_result(conn)
                .to_db_error(ErrorCode::InsertError, "Could not create loot box instance")?;

            TicketInstance::add_to_loot_box_instance(current_user_id, instance.id, loot_box_content.event_id, loot_box_content.min_rarity_id, loot_box_content.max_rarity_id, quantity, conn)?;
        }

        Ok(())
    }

//    pub fn reserve
}

impl NewLootBoxInstance{
    pub fn commit(self, conn: &PgConnection) -> Result<LootBoxInstance, DatabaseError> {
        diesel::insert_into(loot_box_instances::table).values(self).get_result(conn).to_db_error(ErrorCode::InsertError, "Could not create loot box instance")
    }
}


