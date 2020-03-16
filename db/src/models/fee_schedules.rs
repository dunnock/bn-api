use chrono::NaiveDateTime;
use diesel;
use diesel::prelude::*;
use models::*;
use schema::{fee_schedule_ranges, fee_schedules};
use utils::errors::ConvertToDatabaseError;
use utils::errors::DatabaseError;
use utils::errors::ErrorCode;
use uuid::Uuid;

#[derive(Queryable, Identifiable, Clone, Debug, PartialEq, Serialize)]
pub struct FeeSchedule {
    pub id: Uuid,
    pub name: String,
    pub version: i16,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub organization_id: Uuid,
}

impl FeeSchedule {
    pub fn create(organization_id: Uuid, name: String, ranges: Vec<NewFeeScheduleRange>) -> NewFeeSchedule {
        NewFeeSchedule {
            organization_id,
            name,
            ranges,
        }
    }

    pub fn ranges(&self, conn: &PgConnection) -> Result<Vec<FeeScheduleRange>, DatabaseError> {
        fee_schedule_ranges::table
            .filter(fee_schedule_ranges::fee_schedule_id.eq(self.id))
            .order_by(fee_schedule_ranges::min_price_in_cents.asc())
            .load(conn)
            .to_db_error(ErrorCode::QueryError, "Could not load fee schedule ranges")
    }

    pub fn find(id: Uuid, conn: &PgConnection) -> Result<FeeSchedule, DatabaseError> {
        fee_schedules::table
            .find(id)
            .first::<FeeSchedule>(conn)
            .to_db_error(ErrorCode::QueryError, "Error loading Fee Schedule")
    }

    pub fn find_for_organization(
        organization_id: Uuid,
        conn: &PgConnection,
    ) -> Result<Vec<FeeSchedule>, DatabaseError> {
        fee_schedules::table
            .filter(fee_schedules::organization_id.eq(organization_id))
            .order_by(fee_schedules::created_at.asc())
            .get_results::<FeeSchedule>(conn)
            .to_db_error(ErrorCode::QueryError, "Error loading Fee Schedules")
    }

    pub fn get_range(&self, price: i64, conn: &PgConnection) -> Result<FeeScheduleRange, DatabaseError> {
        let ranges: Vec<FeeScheduleRange> = fee_schedule_ranges::table
            .filter(fee_schedule_ranges::fee_schedule_id.eq(self.id))
            .order_by(fee_schedule_ranges::min_price_in_cents.asc())
            .load(conn)
            .to_db_error(ErrorCode::QueryError, "Could not load fee schedule ranges")?;

        let mut found_range = None;

        for r in 0..ranges.len() {
            if ranges[r].min_price_in_cents > price {
                break;
            }
            found_range = Some(ranges[r].clone());
        }

        match found_range {
            Some(f) => Ok(f),
            None => DatabaseError::no_results("Could not find a valid fee for this price"),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct NewFeeSchedule {
    pub organization_id: Uuid,
    pub name: String,
    pub ranges: Vec<NewFeeScheduleRange>,
}

impl NewFeeSchedule {
    pub fn commit(self, created_by_user_id: Option<Uuid>, conn: &PgConnection) -> Result<FeeSchedule, DatabaseError> {
        let previous_version = fee_schedules::table
            .filter(fee_schedules::name.eq(&self.name))
            .order_by(fee_schedules::version.desc())
            .first::<FeeSchedule>(conn)
            .optional()
            .to_db_error(ErrorCode::QueryError, "Error loading Fee Schedule")?;

        let next_version = match previous_version {
            None => 0,

            Some(pv) => pv.version + 1,
        };

        let result: FeeSchedule = diesel::insert_into(fee_schedules::table)
            .values((
                fee_schedules::name.eq(&self.name),
                fee_schedules::version.eq(next_version),
            ))
            .get_result(conn)
            .to_db_error(ErrorCode::InsertError, "Could not create fee schedule")?;

        #[derive(Insertable)]
        #[table_name = "fee_schedule_ranges"]
        struct I {
            fee_schedule_id: Uuid,
            min_price_in_cents: i64,
            fee_in_cents: i64,
            company_fee_in_cents: i64,
            client_fee_in_cents: i64,
        }
        let mut ranges = Vec::<I>::new();
        for range in &self.ranges {
            ranges.push(I {
                fee_schedule_id: result.id,
                min_price_in_cents: range.min_price_in_cents,
                fee_in_cents: range.company_fee_in_cents + range.client_fee_in_cents,
                company_fee_in_cents: range.company_fee_in_cents,
                client_fee_in_cents: range.client_fee_in_cents,
            })
        }
        diesel::insert_into(fee_schedule_ranges::table)
            .values(ranges)
            .execute(conn)
            .to_db_error(ErrorCode::InsertError, "Could not create fee schedule range")?;

        DomainEvent::create(
            DomainEventTypes::FeeScheduleCreated,
            "Fee schedule created".to_string(),
            Tables::FeeSchedules,
            Some(result.id),
            created_by_user_id,
            None,
        )
        .commit(conn)?;

        Ok(result)
    }
}
