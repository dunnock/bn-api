use super::{Event, EventEditableAttributes, NewEvent};
use chrono::prelude::*;
use diesel::deserialize::{self, FromSql};
use diesel::pg::{Pg, PgConnection};
use diesel::query_dsl::{QueryDsl, RunQueryDsl};
use diesel::serialize::{self, Output, ToSql};
use diesel::sql_types::Jsonb;
use diesel::ExpressionMethods;
use models::{EventOverrideStatus, EventStatus, EventTypes, Organization, Venue};
use schema::events;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::io::Write;
use utils::errors::*;
use uuid::Uuid;

#[derive(Default, FromSqlRow, AsExpression, Debug, Serialize, Deserialize, Clone, PartialEq)]
#[sql_type = "Jsonb"]
pub(crate) struct EventAdditionalJson {
    pub cover_image_url: Option<String>,
    pub video_url: Option<String>,
    pub top_line_info: Option<String>,
    pub additional_info: Option<String>,
    pub promo_image_url: Option<String>,
}

impl FromSql<Jsonb, Pg> for EventAdditionalJson {
    fn from_sql(bytes: Option<&[u8]>) -> deserialize::Result<Self> {
        let value = <serde_json::Value as FromSql<Jsonb, Pg>>::from_sql(bytes)?;
        Ok(serde_json::from_value(value)?)
    }
}

impl ToSql<Jsonb, Pg> for EventAdditionalJson {
    fn to_sql<W: Write>(&self, out: &mut Output<W, Pg>) -> serialize::Result {
        let value = serde_json::to_value(self)?;
        <Value as ToSql<Jsonb, Pg>>::to_sql(&value, out)
    }
}

impl From<&Event> for EventAdditionalJson {
    fn from(event: &Event) -> Self {
        Self {
            cover_image_url: event.cover_image_url.clone(),
            video_url: event.video_url.clone(),
            top_line_info: event.top_line_info.clone(),
            additional_info: event.additional_info.clone(),
            promo_image_url: event.promo_image_url.clone(),
        }
    }
}

impl From<&NewEvent> for EventAdditionalJson {
    fn from(event: &NewEvent) -> Self {
        Self {
            cover_image_url: event.cover_image_url.clone(),
            video_url: event.video_url.clone(),
            top_line_info: event.top_line_info.clone(),
            additional_info: event.additional_info.clone(),
            promo_image_url: event.promo_image_url.clone(),
        }
    }
}

#[derive(Identifiable, Serialize, Deserialize, Debug, PartialEq)]
#[table_name = "events"]
pub(crate) struct EventId {
    pub id: Uuid,
}

impl From<&Event> for EventId {
    fn from(event: &Event) -> Self {
        Self { id: event.id }
    }
}

#[derive(Associations, Identifiable, Queryable, QueryableByName)]
#[belongs_to(Organization)]
#[derive(Serialize, Deserialize, Debug)]
#[belongs_to(Venue)]
#[table_name = "events"]
pub(crate) struct EventData {
    pub id: Uuid,
    pub name: String,
    pub organization_id: Uuid,
    pub venue_id: Option<Uuid>,
    pub created_at: NaiveDateTime,
    pub event_start: Option<NaiveDateTime>,
    pub door_time: Option<NaiveDateTime>,
    pub status: EventStatus,
    pub publish_date: Option<NaiveDateTime>,
    pub redeem_date: Option<NaiveDateTime>,
    pub age_limit: Option<String>,
    pub cancelled_at: Option<NaiveDateTime>,
    pub updated_at: NaiveDateTime,
    pub is_external: bool,
    pub external_url: Option<String>,
    pub override_status: Option<EventOverrideStatus>,
    pub client_fee_in_cents: Option<i64>,
    pub company_fee_in_cents: Option<i64>,
    pub settlement_amount_in_cents: Option<i64>,
    pub event_end: Option<NaiveDateTime>,
    pub sendgrid_list_id: Option<i64>,
    pub event_type: EventTypes,
    pub private_access_code: Option<String>,
    pub facebook_pixel_key: Option<String>,
    pub deleted_at: Option<NaiveDateTime>,
    pub extra_admin_data: Option<Value>,
    pub slug_id: Option<Uuid>,
    pub facebook_event_id: Option<String>,
    pub settled_at: Option<NaiveDateTime>,
    pub cloned_from_event_id: Option<Uuid>,
    pub additional_json: EventAdditionalJson,
}

impl From<EventData> for Event {
    fn from(event: EventData) -> Self {
        Self {
            id: event.id,
            name: event.name,
            organization_id: event.organization_id,
            venue_id: event.venue_id,
            created_at: event.created_at,
            event_start: event.event_start,
            door_time: event.door_time,
            status: event.status,
            publish_date: event.publish_date,
            redeem_date: event.redeem_date,
            age_limit: event.age_limit,
            cancelled_at: event.cancelled_at,
            updated_at: event.updated_at,
            is_external: event.is_external,
            external_url: event.external_url,
            override_status: event.override_status,
            client_fee_in_cents: event.client_fee_in_cents,
            company_fee_in_cents: event.company_fee_in_cents,
            settlement_amount_in_cents: event.settlement_amount_in_cents,
            event_end: event.event_end,
            sendgrid_list_id: event.sendgrid_list_id,
            event_type: event.event_type,
            private_access_code: event.private_access_code,
            facebook_pixel_key: event.facebook_pixel_key,
            deleted_at: event.deleted_at,
            extra_admin_data: event.extra_admin_data,
            slug_id: event.slug_id,
            facebook_event_id: event.facebook_event_id,
            settled_at: event.settled_at,
            cloned_from_event_id: event.cloned_from_event_id,
            cover_image_url: event.additional_json.cover_image_url,
            video_url: event.additional_json.video_url,
            top_line_info: event.additional_json.top_line_info,
            additional_info: event.additional_json.additional_info,
            promo_image_url: event.additional_json.promo_image_url,
        }
    }
}

impl From<Event> for EventData {
    fn from(event: Event) -> Self {
        let additional_json = EventAdditionalJson::from(&event);
        Self {
            id: event.id,
            name: event.name,
            organization_id: event.organization_id,
            venue_id: event.venue_id,
            created_at: event.created_at,
            event_start: event.event_start,
            door_time: event.door_time,
            status: event.status,
            publish_date: event.publish_date,
            redeem_date: event.redeem_date,
            age_limit: event.age_limit,
            cancelled_at: event.cancelled_at,
            updated_at: event.updated_at,
            is_external: event.is_external,
            external_url: event.external_url,
            override_status: event.override_status,
            client_fee_in_cents: event.client_fee_in_cents,
            company_fee_in_cents: event.company_fee_in_cents,
            settlement_amount_in_cents: event.settlement_amount_in_cents,
            event_end: event.event_end,
            sendgrid_list_id: event.sendgrid_list_id,
            event_type: event.event_type,
            private_access_code: event.private_access_code,
            facebook_pixel_key: event.facebook_pixel_key,
            deleted_at: event.deleted_at,
            extra_admin_data: event.extra_admin_data,
            slug_id: event.slug_id,
            facebook_event_id: event.facebook_event_id.clone(),
            settled_at: event.settled_at,
            cloned_from_event_id: event.cloned_from_event_id,
            additional_json,
        }
    }
}

impl EventData {
    pub(crate) fn vec_into_events(events: Vec<EventData>) -> Vec<Event> {
        events.into_iter().map(|event| event.into()).collect()
    }
}

#[derive(Insertable, Serialize)]
#[table_name = "events"]
pub(crate) struct NewEventData {
    pub name: String,
    pub organization_id: Uuid,
    pub venue_id: Option<Uuid>,
    pub event_start: Option<NaiveDateTime>,
    pub door_time: Option<NaiveDateTime>,
    pub status: EventStatus,
    pub publish_date: Option<NaiveDateTime>,
    pub redeem_date: Option<NaiveDateTime>,
    pub age_limit: Option<String>,
    pub is_external: bool,
    pub external_url: Option<String>,
    pub override_status: Option<EventOverrideStatus>,
    pub event_end: Option<NaiveDateTime>,
    pub event_type: EventTypes,
    pub private_access_code: Option<String>,
    pub facebook_pixel_key: Option<String>,
    pub extra_admin_data: Option<Value>,
    pub facebook_event_id: Option<String>,
    pub cloned_from_event_id: Option<Uuid>,
    pub additional_json: EventAdditionalJson,
}

impl From<NewEvent> for NewEventData {
    fn from(event: NewEvent) -> Self {
        let additional_json = EventAdditionalJson::from(&event);
        Self {
            name: event.name,
            organization_id: event.organization_id,
            venue_id: event.venue_id,
            event_start: event.event_start,
            door_time: event.door_time,
            status: event.status,
            publish_date: event.publish_date,
            redeem_date: event.redeem_date,
            age_limit: event.age_limit,
            is_external: event.is_external,
            external_url: event.external_url,
            override_status: event.override_status,
            event_end: event.event_end,
            event_type: event.event_type,
            private_access_code: event.private_access_code,
            facebook_pixel_key: event.facebook_pixel_key,
            extra_admin_data: event.extra_admin_data,
            facebook_event_id: event.facebook_event_id.clone(),
            cloned_from_event_id: event.cloned_from_event_id,
            additional_json,
        }
    }
}

#[derive(AsChangeset, Default, Serialize)]
#[table_name = "events"]
pub(crate) struct EventEditableAttributesData {
    pub name: Option<String>,
    pub venue_id: Option<Uuid>,
    pub event_start: Option<NaiveDateTime>,
    pub door_time: Option<NaiveDateTime>,
    pub publish_date: Option<Option<NaiveDateTime>>,
    pub redeem_date: Option<NaiveDateTime>,
    pub age_limit: Option<String>,
    pub cancelled_at: Option<NaiveDateTime>,
    pub is_external: Option<bool>,
    pub external_url: Option<Option<String>>,
    pub override_status: Option<Option<EventOverrideStatus>>,
    pub event_end: Option<NaiveDateTime>,
    pub private_access_code: Option<Option<String>>,
    pub sendgrid_list_id: Option<i64>,
    pub event_type: Option<EventTypes>,
    pub facebook_pixel_key: Option<Option<String>>,
    pub facebook_event_id: Option<Option<String>>,
    pub cloned_from_event_id: Option<Option<Uuid>>,
    pub additional_json: Option<EventAdditionalJson>,
}

impl EventEditableAttributesData {
    pub fn prepare_update(
        id: Uuid,
        event: EventEditableAttributes,
        conn: &PgConnection,
    ) -> Result<EventEditableAttributesData, DatabaseError> {
        let additional_json = Self::select_and_merge(id, &event, conn)?;

        Ok(Self {
            name: event.name,
            venue_id: event.venue_id,
            event_start: event.event_start,
            door_time: event.door_time,
            publish_date: event.publish_date,
            redeem_date: event.redeem_date,
            age_limit: event.age_limit,
            is_external: event.is_external,
            external_url: event.external_url,
            override_status: event.override_status,
            event_end: event.event_end,
            event_type: event.event_type,
            private_access_code: event.private_access_code,
            facebook_pixel_key: event.facebook_pixel_key,
            facebook_event_id: event.facebook_event_id,
            cloned_from_event_id: event.cloned_from_event_id,
            cancelled_at: event.cancelled_at,
            sendgrid_list_id: event.sendgrid_list_id,
            additional_json,
        })
    }

    fn select_and_merge(
        id: Uuid,
        event: &EventEditableAttributes,
        conn: &PgConnection,
    ) -> Result<Option<EventAdditionalJson>, DatabaseError> {
        if event.cover_image_url.is_none()
            && event.video_url.is_none()
            && event.top_line_info.is_none()
            && event.additional_info.is_none()
            && event.promo_image_url.is_none()
        {
            return Ok(None);
        };

        let mut changed = false;

        let mut current: EventAdditionalJson = events::table
            .filter(events::id.eq(id))
            .select(events::additional_json)
            .get_result(conn)
            .to_db_error(ErrorCode::QueryError, "Could not load additional_json for event")?;

        macro_rules! check_and_update {
            ($field:ident) => {
                if event.$field.is_some() {
                    let new_value = event.$field.clone().unwrap();
                    if current.$field != new_value {
                        current.$field = new_value;
                        changed = true;
                    }
                }
            };
        }

        check_and_update!(cover_image_url);
        check_and_update!(video_url);
        check_and_update!(top_line_info);
        check_and_update!(additional_info);
        check_and_update!(promo_image_url);

        if changed {
            Ok(Some(current))
        } else {
            Ok(None)
        }
    }
}
