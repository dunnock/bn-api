use chrono::{Duration, NaiveDateTime, Utc};
use diesel;
use diesel::expression::dsl;
use diesel::prelude::*;
use models::enums::*;
use schema::*;
use serde_json;
use utils::dates;
use utils::dates::IntoDateBuilder;
use utils::errors::*;
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, PartialEq, Identifiable, Queryable, QueryableByName)]
#[table_name = "domain_actions"]
pub struct DomainAction {
    pub id: Uuid,
    pub domain_event_id: Option<Uuid>,
    pub domain_action_type: DomainActionTypes,
    pub communication_channel_type: Option<CommunicationChannelType>,
    pub payload: serde_json::Value,
    pub main_table: Option<Tables>,
    pub main_table_id: Option<Uuid>,
    pub scheduled_at: NaiveDateTime,
    pub expires_at: NaiveDateTime,
    pub last_attempted_at: Option<NaiveDateTime>,
    pub attempt_count: i64,
    pub max_attempt_count: i64,
    pub status: DomainActionStatus,
    pub last_failure_reason: Option<String>,
    pub blocked_until: NaiveDateTime,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(AsChangeset, Deserialize)]
#[table_name = "domain_actions"]
pub struct DomainActionEditableAttributes {
    pub scheduled_at: Option<NaiveDateTime>,
    pub last_attempted_at: Option<NaiveDateTime>,
    pub attempt_count: Option<i64>,
    pub blocked_until: NaiveDateTime,
}

impl DomainAction {
    pub fn upcoming_domain_action(
        table: Option<Tables>,
        id: Option<Uuid>,
        domain_action_type: DomainActionTypes,
        conn: &PgConnection,
    ) -> Result<Option<DomainAction>, DatabaseError> {
        Ok(DomainAction::find_by_resource(table, id, domain_action_type, DomainActionStatus::Pending, conn)?.pop())
    }

    pub fn create(
        domain_event_id: Option<Uuid>,
        domain_action_type: DomainActionTypes,
        communication_channel_type: Option<CommunicationChannelType>,
        payload: serde_json::Value,
        main_table: Option<Tables>,
        main_table_id: Option<Uuid>,
    ) -> NewDomainAction {
        NewDomainAction {
            domain_event_id,
            domain_action_type,
            communication_channel_type,
            payload,
            main_table,
            main_table_id,
            scheduled_at: Utc::now().naive_utc(),
            expires_at: dates::now().add_seconds(900).finish(),
            last_attempted_at: None,
            attempt_count: 0,
            max_attempt_count: 3,
            status: DomainActionStatus::Pending,
            blocked_until: dates::now().add_seconds(-30).finish(),
        }
    }

    pub fn find_stuck(conn: &PgConnection) -> Result<Vec<DomainAction>, DatabaseError> {
        let sql = r#"
            SELECT *
            FROM domain_actions
            WHERE status = 'Pending'
            AND blocked_until + interval '1 minute' < current_timestamp
            ORDER BY created_at desc;"#;
        let result: Vec<DomainAction> = diesel::sql_query(sql)
            .get_results(conn)
            .to_db_error(ErrorCode::QueryError, "Could not find stuck domain actions")?;
        Ok(result)
    }

    pub fn find_by_resource(
        main_table: Option<Tables>,
        main_table_id: Option<Uuid>,
        domain_action_type: DomainActionTypes,
        domain_action_status: DomainActionStatus,
        conn: &PgConnection,
    ) -> Result<Vec<DomainAction>, DatabaseError> {
        let now = Utc::now().naive_utc();
        let mut query = domain_actions::table
            .select(domain_actions::all_columns)
            .filter(domain_actions::status.eq(domain_action_status))
            .filter(domain_actions::domain_action_type.eq(domain_action_type))
            .into_boxed();

        if let Some(main_table) = main_table {
            query = query.filter(domain_actions::main_table.eq(main_table));
        }

        if let Some(main_table_id) = main_table_id {
            query = query.filter(domain_actions::main_table_id.eq(main_table_id));
        }

        query
            .filter(domain_actions::expires_at.gt(now))
            .filter(domain_actions::attempt_count.lt(domain_actions::max_attempt_count))
            .load(conn)
            .to_db_error(ErrorCode::QueryError, "Error loading domain actions")
    }

    pub fn find_pending(
        domain_action_type: Option<DomainActionTypes>,
        conn: &PgConnection,
    ) -> Result<Vec<DomainAction>, DatabaseError> {
        // Use server time as opposed to db time because
        // the actions will be created using server time
        // and if the two are out of sync, some tests will
        // not find the action
        let now = Utc::now().naive_utc();
        let mut query = domain_actions::table
            .filter(domain_actions::scheduled_at.le(now))
            .filter(domain_actions::expires_at.gt(now))
            .filter(domain_actions::blocked_until.le(now))
            .filter(domain_actions::attempt_count.lt(domain_actions::max_attempt_count))
            .filter(domain_actions::status.eq(DomainActionStatus::Pending))
            .into_boxed();

        if let Some(action_type) = domain_action_type {
            query = query.filter(domain_actions::domain_action_type.eq(action_type));
        }

        query
            .load(conn)
            .to_db_error(ErrorCode::QueryError, "Error loading domain actions")
    }

    /// This method returns true if a pending/busy domain action
    /// exists for the given `domain_action_type`, `main_table` and `main_table_id`
    /// otherwise false.
    pub fn has_pending_action(
        action_type: DomainActionTypes,
        main_table: Tables,
        main_table_id: Uuid,
        conn: &PgConnection,
    ) -> Result<bool, DatabaseError> {
        domain_actions::table
            .select(dsl::count(domain_actions::id))
            .filter(domain_actions::domain_action_type.eq(action_type))
            .filter(domain_actions::status.eq(DomainActionStatus::Pending))
            .filter(domain_actions::expires_at.gt(dsl::now))
            .filter(domain_actions::main_table.eq(main_table))
            .filter(domain_actions::main_table_id.eq(main_table_id))
            .limit(1)
            .get_result(conn)
            .map(|count: i64| count > 0)
            .to_db_error(ErrorCode::QueryError, "Error loading domain actions")
    }

    pub fn find(id: Uuid, conn: &PgConnection) -> Result<DomainAction, DatabaseError> {
        DatabaseError::wrap(
            ErrorCode::QueryError,
            "Error loading domain action",
            domain_actions::table.find(id).first::<DomainAction>(conn),
        )
    }

    pub fn set_busy(&self, timeout: i64, conn: &PgConnection) -> Result<(), DatabaseError> {
        let timeout = Utc::now().naive_utc() + Duration::seconds(timeout);
        let db_blocked = DomainAction::find(self.id, conn)?;
        if db_blocked.blocked_until > Utc::now().naive_utc() {
            return DatabaseError::concurrency_error("Another process is busy with this action");
        };
        let result: Option<DomainAction> = diesel::update(self)
            .filter(domain_actions::blocked_until.le(dsl::now))
            .set((
                domain_actions::blocked_until.eq(timeout),
                domain_actions::updated_at.eq(dsl::now),
            ))
            .get_result(conn)
            .to_db_error(ErrorCode::UpdateError, "Could not update Domain Action")
            .optional()?;

        match result {
            Some(_) => Ok(()),
            None => DatabaseError::concurrency_error("Another process is busy with this action"),
        }
    }

    pub fn set_cancelled(&self, conn: &PgConnection) -> Result<DomainAction, DatabaseError> {
        diesel::update(self)
            .set((
                domain_actions::status.eq(DomainActionStatus::Cancelled),
                domain_actions::updated_at.eq(dsl::now),
            ))
            .get_result(conn)
            .to_db_error(ErrorCode::UpdateError, "Could not update Domain Action")
    }

    pub fn set_done(&self, conn: &PgConnection) -> Result<DomainAction, DatabaseError> {
        diesel::update(self)
            .set((
                domain_actions::status.eq(DomainActionStatus::Success),
                domain_actions::updated_at.eq(dsl::now),
            ))
            .get_result(conn)
            .to_db_error(ErrorCode::UpdateError, "Could not update Domain Action")
    }

    /// Use this method if there was a transient failure in performing the action. In
    /// general, it is assumed that the action will succeed at a later stage. If the
    /// action should not be retried, use `errored` instead. If the number of retries
    /// is exceeded, the status will changed to `RetriedExceeded`.
    pub fn set_failed(&self, reason: &str, conn: &PgConnection) -> Result<DomainAction, DatabaseError> {
        if self.max_attempt_count <= self.attempt_count + 1 {
            diesel::update(self)
                .set((
                    domain_actions::last_failure_reason.eq(reason),
                    domain_actions::status.eq(DomainActionStatus::RetriesExceeded),
                    domain_actions::attempt_count.eq(self.attempt_count + 1),
                    domain_actions::blocked_until.eq(dsl::now),
                    domain_actions::updated_at.eq(dsl::now),
                ))
                .get_result(conn)
                .to_db_error(ErrorCode::UpdateError, "Could not update Domain Action")
        } else {
            // Intentionally leave checked out
            diesel::update(self)
                .set((
                    domain_actions::last_failure_reason.eq(reason),
                    domain_actions::attempt_count.eq(self.attempt_count + 1),
                    domain_actions::updated_at.eq(dsl::now),
                ))
                .get_result(conn)
                .to_db_error(ErrorCode::UpdateError, "Could not update Domain Action")
        }
    }

    /// Call this method to indicate that the action has errored and should not be retried.
    /// If there is a chance that the action could succeed at a later stage, use `failed()`
    /// instead
    pub fn set_errored(&self, reason: &str, conn: &PgConnection) -> Result<DomainAction, DatabaseError> {
        diesel::update(self)
            .set((
                domain_actions::last_failure_reason.eq(reason),
                domain_actions::status.eq(DomainActionStatus::Errored),
                domain_actions::blocked_until.eq(dsl::now),
                domain_actions::updated_at.eq(dsl::now),
            ))
            .get_result(conn)
            .to_db_error(ErrorCode::UpdateError, "Could not update Domain Action")
    }

    pub fn set_scheduled_at(
        &self,
        scheduled_at: NaiveDateTime,
        conn: &PgConnection,
    ) -> Result<DomainAction, DatabaseError> {
        let date_builder = scheduled_at.clone().into_builder();

        let expires_at = date_builder.clone().add_seconds(900).finish();
        let blocked_until = date_builder.add_seconds(-30).finish();
        diesel::update(self)
            .set((
                domain_actions::scheduled_at.eq(scheduled_at),
                domain_actions::expires_at.eq(expires_at),
                domain_actions::blocked_until.eq(blocked_until),
                domain_actions::updated_at.eq(dsl::now),
            ))
            .get_result(conn)
            .to_db_error(ErrorCode::UpdateError, "Could not update Domain Action")
    }

    pub fn update(
        &self,
        attributes: &DomainActionEditableAttributes,
        conn: &PgConnection,
    ) -> Result<DomainAction, DatabaseError> {
        diesel::update(self)
            .set((attributes, domain_actions::updated_at.eq(dsl::now)))
            .get_result(conn)
            .to_db_error(ErrorCode::UpdateError, "Could not update Domain Action")
    }
}

#[derive(Clone, Insertable)]
#[table_name = "domain_actions"]
pub struct NewDomainAction {
    pub domain_event_id: Option<Uuid>,
    pub domain_action_type: DomainActionTypes,
    pub communication_channel_type: Option<CommunicationChannelType>,
    pub payload: serde_json::Value,
    pub main_table: Option<Tables>,
    pub main_table_id: Option<Uuid>,
    pub scheduled_at: NaiveDateTime,
    pub expires_at: NaiveDateTime,
    pub last_attempted_at: Option<NaiveDateTime>,
    pub attempt_count: i64,
    pub max_attempt_count: i64,
    pub status: DomainActionStatus,
    pub blocked_until: NaiveDateTime,
}

impl NewDomainAction {
    pub fn commit(self, conn: &PgConnection) -> Result<DomainAction, DatabaseError> {
        diesel::insert_into(domain_actions::table)
            .values(self)
            .get_result(conn)
            .to_db_error(ErrorCode::InsertError, "Could not insert domain message")
    }

    pub fn schedule_at(&mut self, schedule_at: NaiveDateTime) {
        let diff = schedule_at - self.scheduled_at;
        self.scheduled_at = schedule_at;
        self.expires_at = self.expires_at + diff;
        self.blocked_until = self.blocked_until + diff;
    }
}
