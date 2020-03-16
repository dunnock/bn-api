use chrono::{Duration, NaiveDateTime, Utc};
use diesel;
use diesel::expression::dsl;
use diesel::prelude::*;
use models::*;
use schema::{domain_event_published, domain_event_publishers};
use serde_json::Value;
use std::hash::{Hash, Hasher};
use utils::errors::*;
use uuid::Uuid;
use validator::Validate;

pub static SUPPORTED_DOMAIN_EVENT_TYPES_FOR_PUBLISHING: &'static [DomainEventTypes] = &[
    DomainEventTypes::TransferTicketStarted,
    DomainEventTypes::TransferTicketCancelled,
    DomainEventTypes::TransferTicketCompleted,
    DomainEventTypes::UserCreated,
    DomainEventTypes::OrderCompleted,
    DomainEventTypes::OrderRefund,
    DomainEventTypes::OrderResendConfirmationTriggered,
    DomainEventTypes::OrderRetargetingEmailTriggered,
    DomainEventTypes::TemporaryUserCreated,
    DomainEventTypes::PushNotificationTokenCreated,
];

#[derive(Clone, Debug, Serialize, Identifiable, Queryable, QueryableByName)]
#[table_name = "domain_event_publishers"]
pub struct DomainEventPublisher {
    pub id: Uuid,
    pub organization_id: Option<Uuid>,
    pub event_types: Vec<DomainEventTypes>,
    pub webhook_url: String,
    pub import_historic_events: bool,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub last_domain_event_seq: Option<i64>,
    pub deleted_at: Option<NaiveDateTime>,
    pub adapter: Option<WebhookAdapters>,
    pub adapter_config: Option<Value>,
    pub blocked_until: NaiveDateTime,
}

impl Eq for DomainEventPublisher {}

impl PartialEq for DomainEventPublisher {
    fn eq(&self, other: &DomainEventPublisher) -> bool {
        self.id.eq(&other.id)
    }
}

impl Hash for DomainEventPublisher {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state)
    }
}

#[derive(AsChangeset, Deserialize, Validate)]
#[table_name = "domain_event_publishers"]
pub struct DomainEventPublisherEditableAttributes {
    #[validate(url(message = "Webhook URL is invalid"))]
    pub webhook_url: Option<String>,
    pub import_historic_events: Option<bool>,
}

impl DomainEventPublisher {
    pub fn find_all(conn: &PgConnection) -> Result<Vec<DomainEventPublisher>, DatabaseError> {
        domain_event_publishers::table
            .filter(domain_event_publishers::deleted_at.is_null())
            .order_by(domain_event_publishers::last_domain_event_seq.asc())
            .load(conn)
            .to_db_error(ErrorCode::QueryError, "Could not load Domain Event Publishers")
    }

    pub fn delete(self, conn: &PgConnection) -> Result<(), DatabaseError> {
        diesel::update(&self)
            .set((
                domain_event_publishers::deleted_at.eq(dsl::now.nullable()),
                domain_event_publishers::updated_at.eq(dsl::now),
            ))
            .execute(conn)
            .to_db_error(ErrorCode::UpdateError, "Could not delete domain event publisher")?;

        Ok(())
    }

    pub fn claim_for_publishing(&self, domain_event: &DomainEvent, conn: &PgConnection) -> Result<bool, DatabaseError> {
        // Mark domain event published for this publisher
        diesel::insert_into(domain_event_published::table)
            .values((
                domain_event_published::domain_event_publisher_id.eq(self.id),
                domain_event_published::domain_event_id.eq(domain_event.id),
            ))
            .execute(conn)
            .to_db_error(ErrorCode::InsertError, "Could not insert domain event published")?;

        Ok(true)
    }

    pub fn update_last_domain_event_seq(
        &mut self,
        last_domain_event_seq: i64,
        conn: &PgConnection,
    ) -> Result<(), DatabaseError> {
        use diesel::dsl;
        use schema::*;
        let res: Option<DomainEventPublisher> = diesel::update(
            domain_event_publishers::table
                .filter(
                    domain_event_publishers::last_domain_event_seq
                        .is_null()
                        .or(domain_event_publishers::last_domain_event_seq.lt(last_domain_event_seq)),
                )
                .filter(domain_event_publishers::id.eq(self.id)),
        )
        .set((
            domain_event_publishers::last_domain_event_seq.eq(last_domain_event_seq),
            domain_event_publishers::updated_at.eq(dsl::now),
        ))
        .get_result(conn)
        .to_db_error(ErrorCode::UpdateError, "Could not update domain event publisher")
        .optional()?;

        let r = match res {
            Some(r) => Ok(r),
            None =>
            // A later event has already been published....
            {
                DomainEventPublisher::find(self.id, conn)
            }
        }?;

        self.last_domain_event_seq = r.last_domain_event_seq;
        Ok(())
    }

    pub fn create(
        organization_id: Option<Uuid>,
        event_types: Vec<DomainEventTypes>,
        webhook_url: String,
    ) -> NewDomainEventPublisher {
        NewDomainEventPublisher {
            organization_id,
            event_types,
            webhook_url,
            adapter: None,
            adapter_config: None,
        }
    }

    pub fn create_with_adapter(
        organization_id: Option<Uuid>,
        event_types: Vec<DomainEventTypes>,
        adapter: WebhookAdapters,
        adapter_config: Value,
    ) -> NewDomainEventPublisher {
        NewDomainEventPublisher {
            organization_id,
            event_types,
            webhook_url: "".to_string(),
            adapter: Some(adapter),
            adapter_config: Some(adapter_config),
        }
    }

    pub fn find(id: Uuid, conn: &PgConnection) -> Result<DomainEventPublisher, DatabaseError> {
        DatabaseError::wrap(
            ErrorCode::QueryError,
            "Error loading domain event publisher",
            domain_event_publishers::table
                .find(id)
                .first::<DomainEventPublisher>(conn),
        )
    }

    pub fn update(
        &self,
        attributes: &DomainEventPublisherEditableAttributes,
        conn: &PgConnection,
    ) -> Result<DomainEventPublisher, DatabaseError> {
        diesel::update(self)
            .set((attributes, domain_event_publishers::updated_at.eq(dsl::now)))
            .get_result(conn)
            .to_db_error(ErrorCode::UpdateError, "Could not update domain event publisher")
    }

    pub fn acquire_lock(&mut self, timeout: i64, conn: &PgConnection) -> Result<(), DatabaseError> {
        let timeout = Utc::now().naive_utc() + Duration::seconds(timeout);
        let result: Option<DomainEventPublisher> = diesel::update(&*self)
            .filter(domain_event_publishers::blocked_until.le(dsl::now))
            .filter(domain_event_publishers::id.eq(self.id))
            .set((
                domain_event_publishers::blocked_until.eq(timeout),
                domain_event_publishers::updated_at.eq(dsl::now),
            ))
            .get_result(conn)
            .to_db_error(ErrorCode::UpdateError, "Could not update Domain Event Publisher")
            .optional()?;

        match result {
            Some(publisher) => {
                self.blocked_until = publisher.blocked_until;
                self.updated_at = publisher.updated_at;
                Ok(())
            }
            None => DatabaseError::concurrency_error("Another process is busy with this publisher"),
        }
    }

    pub fn renew_lock(&mut self, timeout: i64, conn: &PgConnection) -> Result<(), DatabaseError> {
        let timeout = Utc::now().naive_utc() + Duration::seconds(timeout);
        let result: Option<DomainEventPublisher> = diesel::update(&*self)
            .filter(domain_event_publishers::blocked_until.eq(self.blocked_until))
            .filter(domain_event_publishers::id.eq(self.id))
            .set((
                domain_event_publishers::blocked_until.eq(timeout),
                domain_event_publishers::updated_at.eq(dsl::now),
            ))
            .get_result(conn)
            .to_db_error(ErrorCode::UpdateError, "Could not update Domain Event Publisher")
            .optional()?;

        match result {
            Some(publisher) => {
                self.blocked_until = publisher.blocked_until;
                self.updated_at = publisher.updated_at;
                Ok(())
            }
            None => DatabaseError::concurrency_error("Another process is busy with this publisher"),
        }
    }

    pub fn release_lock(&mut self, conn: &PgConnection) -> Result<(), DatabaseError> {
        let result: Option<DomainEventPublisher> = diesel::update(&*self)
            .filter(domain_event_publishers::blocked_until.eq(self.blocked_until))
            .filter(domain_event_publishers::id.eq(self.id))
            .set((
                domain_event_publishers::blocked_until.eq(dsl::now),
                domain_event_publishers::updated_at.eq(dsl::now),
            ))
            .get_result(conn)
            .to_db_error(ErrorCode::UpdateError, "Could not update Domain Event Publisher")
            .optional()?;

        match result {
            Some(publisher) => {
                self.blocked_until = publisher.blocked_until;
                self.updated_at = publisher.updated_at;
                Ok(())
            },
            None => DatabaseError::concurrency_error(
                "Failed to release lock, another process has acquired a lock on this publisher in the interim. Consider raising the timeout value calling acquire_lock.",
            ),
        }
    }
}

#[derive(Clone, Deserialize, Insertable, Validate)]
#[table_name = "domain_event_publishers"]
pub struct NewDomainEventPublisher {
    pub organization_id: Option<Uuid>,
    pub event_types: Vec<DomainEventTypes>,
    pub webhook_url: String,
    pub adapter: Option<WebhookAdapters>,
    pub adapter_config: Option<Value>,
}

impl NewDomainEventPublisher {
    pub fn commit(self, conn: &PgConnection) -> Result<DomainEventPublisher, DatabaseError> {
        diesel::insert_into(domain_event_publishers::table)
            .values(self)
            .get_result(conn)
            .to_db_error(ErrorCode::InsertError, "Could not insert domain event publisher")
    }
}
