use chrono::{NaiveDateTime, Utc};
use db::prelude::*;
use diesel::PgConnection;
use std::cmp;
use uuid::Uuid;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct DisplayTicketPricing {
    pub id: Uuid,
    pub name: String,
    pub status: TicketPricingStatus,
    pub start_date: NaiveDateTime,
    pub end_date: NaiveDateTime,
    pub price_in_cents: i64,
    pub fee_in_cents: i64,
    pub discount_in_cents: i64,
    pub associated_with_active_orders: bool,
}

impl DisplayTicketPricing {
    pub fn from_ticket_pricing(
        ticket_pricing: &TicketPricing,
        fee_schedule: &FeeSchedule,
        redemption_code: Option<String>,
        event_id: Option<Uuid>,
        box_office_pricing: bool,
        conn: &PgConnection,
    ) -> Result<DisplayTicketPricing, DatabaseError> {
        let mut is_comp = false;
        let mut discount_in_cents = 0;
        if let Some(redemption_code) = redemption_code {
            if let Some(hold) = Hold::find_by_redemption_code(&redemption_code, event_id, conn).optional()? {
                if hold.ticket_type_id == ticket_pricing.ticket_type_id {
                    if hold.hold_type == HoldTypes::Comp {
                        is_comp = true;
                        discount_in_cents = ticket_pricing.price_in_cents;
                    } else {
                        discount_in_cents = hold.discount_in_cents.unwrap_or(0);
                    }
                }
            } else if let Some(code_availability) =
                Code::find_by_redemption_code_with_availability(&redemption_code, event_id, conn).optional()?
            {
                let now = Utc::now().naive_utc();
                if now >= code_availability.code.start_date && now <= code_availability.code.end_date {
                    if TicketType::find_for_code(code_availability.code.id, conn)?
                        .iter()
                        .map(|tt| tt.id)
                        .collect::<Vec<Uuid>>()
                        .contains(&ticket_pricing.ticket_type_id)
                    {
                        if code_availability.code.discount_as_percentage.unwrap_or(0) > 0 {
                            discount_in_cents = (ticket_pricing.price_in_cents as f64
                                * code_availability.code.discount_as_percentage.unwrap_or(0) as f64
                                / 100f64)
                                .floor() as i64;
                        }

                        if code_availability.code.discount_in_cents.unwrap_or(0) > 0 {
                            discount_in_cents = code_availability.code.discount_in_cents.unwrap_or(0);
                        }
                    }
                }
            }
        }

        // Limit reported discount to price of ticket
        discount_in_cents = cmp::min(ticket_pricing.price_in_cents, discount_in_cents);

        // Determine fees using discounted price, comps and box office purchases have no fees
        let mut fee_in_cents = 0;
        if !is_comp && !box_office_pricing {
            fee_in_cents = fee_schedule
                .get_range(ticket_pricing.price_in_cents - discount_in_cents, conn)
                .optional()?
                .map(|f| f.fee_in_cents)
                .unwrap_or(0);
        }

        let associated_with_active_orders = ticket_pricing.associated_with_active_orders(conn)?;
        Ok(DisplayTicketPricing {
            id: ticket_pricing.id,
            name: ticket_pricing.name.clone(),
            status: ticket_pricing.status,
            start_date: ticket_pricing.start_date,
            end_date: ticket_pricing.end_date,
            price_in_cents: ticket_pricing.price_in_cents,
            fee_in_cents,
            discount_in_cents,
            associated_with_active_orders,
        })
    }
}
