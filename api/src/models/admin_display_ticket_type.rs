use bigneon_db::dev::times;
use bigneon_db::prelude::*;
use chrono::NaiveDateTime;
use diesel::PgConnection;
use crate::models::DisplayTicketPricing;
use uuid::Uuid;

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct AdminDisplayTicketType {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub capacity: u32,
    pub status: TicketTypeStatus,
    pub start_date: Option<NaiveDateTime>,
    pub end_date: Option<NaiveDateTime>,
    pub end_date_type: TicketTypeEndDateType,
    pub available: u32,
    pub increment: u32,
    pub limit_per_person: u32,
    pub ticket_pricing: Vec<DisplayTicketPricing>,
    pub price_in_cents: i64,
    pub visibility: TicketTypeVisibility,
    pub parent_id: Option<Uuid>,
    pub parent_name: Option<String>,
    pub additional_fee_in_cents: i64,
    pub rank: i32,
    pub app_sales_enabled: bool,
    pub web_sales_enabled: bool,
    pub box_office_sales_enabled: bool,
}

impl AdminDisplayTicketType {
    pub fn from_ticket_type(
        ticket_type: &TicketType,
        fee_schedule: &FeeSchedule,
        conn: &PgConnection,
    ) -> Result<AdminDisplayTicketType, DatabaseError> {
        let available = ticket_type.valid_available_ticket_count(conn)?;
        let capacity = ticket_type.valid_ticket_count(conn)?;
        let mut ticket_pricing_list = Vec::new();
        let ticket_pricings = ticket_type.valid_ticket_pricing(false, conn)?;
        for ticket_pricing in &ticket_pricings {
            ticket_pricing_list.push(DisplayTicketPricing::from_ticket_pricing(
                &ticket_pricing,
                fee_schedule,
                None,
                Some(ticket_type.event_id),
                false,
                conn,
            )?);
        }

        let parent_name = match ticket_type.parent_id {
            Some(parent_id) => Some(TicketType::find(parent_id, conn)?.name),
            None => None,
        };

        let result = AdminDisplayTicketType {
            id: ticket_type.id,
            name: ticket_type.name.clone(),
            description: ticket_type.description.clone(),
            status: ticket_type.status(false, conn)?,
            start_date: ticket_type
                .start_date
                .and_then(|sd| if sd <= times::zero() { None } else { Some(sd) }),
            parent_id: ticket_type.parent_id,
            parent_name,
            end_date: ticket_type.end_date,
            end_date_type: ticket_type.end_date_type,
            ticket_pricing: ticket_pricing_list.clone(),
            available,
            capacity,
            increment: ticket_type.increment as u32,
            limit_per_person: ticket_type.limit_per_person as u32,
            price_in_cents: ticket_type.price_in_cents,
            visibility: ticket_type.visibility,
            additional_fee_in_cents: ticket_type.additional_fee_in_cents,
            rank: ticket_type.rank,
            app_sales_enabled: ticket_type.app_sales_enabled,
            web_sales_enabled: ticket_type.web_sales_enabled,
            box_office_sales_enabled: ticket_type.box_office_sales_enabled,
        };
        Ok(result)
    }
}
