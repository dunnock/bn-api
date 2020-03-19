use crate::models::DisplayTicketPricing;
use chrono::{NaiveDateTime, Utc};
use db::prelude::*;
use diesel::PgConnection;
use uuid::Uuid;

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct UserDisplayTicketType {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub status: TicketTypeStatus,
    pub available: u32,
    pub start_date: Option<NaiveDateTime>,
    pub end_date: NaiveDateTime,
    pub increment: i32,
    pub limit_per_person: u32,
    pub ticket_pricing: Option<DisplayTicketPricing>,
    pub redemption_code: Option<String>,
    pub event_id: Uuid,
    pub rank: i32,
}

impl UserDisplayTicketType {
    pub fn from_ticket_type(
        ticket_type: &TicketType,
        fee_schedule: &FeeSchedule,
        box_office_pricing: bool,
        redemption_code: Option<String>,
        conn: &PgConnection,
    ) -> Result<UserDisplayTicketType, DatabaseError> {
        let available = ticket_type.valid_available_ticket_count(conn)?;

        let ticket_pricing = match ticket_type
            .current_ticket_pricing(box_office_pricing, conn)
            .optional()?
        {
            Some(ticket_pricing) => Some(DisplayTicketPricing::from_ticket_pricing(
                &ticket_pricing,
                fee_schedule,
                redemption_code.clone(),
                Some(ticket_type.event_id),
                box_office_pricing,
                conn,
            )?),
            None => None,
        };

        let mut result = UserDisplayTicketType {
            id: ticket_type.id,
            event_id: ticket_type.event_id,
            name: ticket_type.name.clone(),
            description: ticket_type.description.clone(),
            status: ticket_type.status(box_office_pricing, conn)?,
            start_date: ticket_type.start_date,
            end_date: ticket_type.end_date(conn)?,
            ticket_pricing,
            available,
            redemption_code: None,
            increment: ticket_type.increment,
            limit_per_person: ticket_type.limit_per_person as u32,
            rank: ticket_type.rank,
        };

        if let Some(ref redemption_code) = redemption_code {
            if let Some(hold) =
                Hold::find_by_redemption_code(redemption_code, Some(ticket_type.event_id), conn).optional()?
            {
                if hold.ticket_type_id == ticket_type.id {
                    result.description = Some(format!("Using promo code: {}", redemption_code));
                    let hold_limit_per_person = hold.max_per_user.unwrap_or(0) as u32;
                    // Limited by the minimum of hold max_per_user and ticket_type limit_per_person with 0 acting as no limit
                    if result.limit_per_person == 0 || result.limit_per_person > hold_limit_per_person {
                        result.limit_per_person = hold_limit_per_person;
                    }
                    result.available = hold.quantity(conn)?.1;
                    result.redemption_code = Some(redemption_code.clone());
                }
            } else if let Some(code_availability) =
                Code::find_by_redemption_code_with_availability(redemption_code, Some(ticket_type.event_id), conn)
                    .optional()?
            {
                let now = Utc::now().naive_utc();
                if now >= code_availability.code.start_date && now <= code_availability.code.end_date {
                    if TicketType::find_for_code(code_availability.code.id, conn)?.contains(&ticket_type) {
                        result.description = Some(format!("Using promo code: {}", redemption_code));
                        let code_limit_per_person = code_availability.code.max_tickets_per_user.unwrap_or(0) as u32;
                        // Limited by the minimum of code max_per_user and ticket_type limit_per_person with 0 acting as no limit
                        if result.limit_per_person == 0 || result.limit_per_person > code_limit_per_person {
                            result.limit_per_person = code_limit_per_person;
                        }
                        result.redemption_code = Some(redemption_code.clone());
                    }
                }
            }
        }

        let ticket_pricings = ticket_type.valid_ticket_pricing(true, conn)?;
        match result.status {
            TicketTypeStatus::OnSaleSoon => {
                let min_pricing = ticket_pricings.iter().min_by_key(|p| p.start_date);
                result.ticket_pricing = Some(DisplayTicketPricing::from_ticket_pricing(
                    min_pricing.unwrap(),
                    fee_schedule,
                    redemption_code.clone(),
                    Some(ticket_type.event_id),
                    box_office_pricing,
                    conn,
                )?);
            }
            TicketTypeStatus::SaleEnded => {
                let max_pricing = ticket_pricings.iter().max_by_key(|p| p.end_date);
                result.ticket_pricing = Some(DisplayTicketPricing::from_ticket_pricing(
                    max_pricing.unwrap(),
                    fee_schedule,
                    redemption_code.clone(),
                    Some(ticket_type.event_id),
                    box_office_pricing,
                    conn,
                )?);
            }
            _ => (),
        }

        Ok(result)
    }
}
