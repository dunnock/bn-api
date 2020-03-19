use db::models::*;
use itertools::Itertools;
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct TicketCountReport {
    data: Vec<TicketCountReportRow>,
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct TicketCountReportRow {
    row_name: String,
    daily_sold: i64,
    total_sold: i64,
    held: i64,
    open: i64,
    total_value_in_cents: i64,
}

impl From<TicketSalesAndCounts> for TicketCountReport {
    fn from(sales_and_counts: TicketSalesAndCounts) -> Self {
        let mut data: Vec<TicketCountReportRow> = Vec::new();

        let mut total_sales: HashMap<Uuid, i64> = HashMap::new();
        for (ticket_type_id, sales) in &sales_and_counts.sales.iter().group_by(|ti| ti.ticket_type_id) {
            if let Some(ticket_type_id) = ticket_type_id {
                total_sales.insert(
                    ticket_type_id,
                    sales
                        .map(|s| s.box_office_sales_in_cents + s.online_sales_in_cents)
                        .sum(),
                );
            }
        }

        for (ticket_type_id, counts) in &sales_and_counts.counts.into_iter().group_by(|ti| ti.ticket_type_id) {
            if let Some(ticket_type_id) = ticket_type_id {
                let ticket_type_counts = counts.collect_vec();
                let row_name = ticket_type_counts
                    .first()
                    .map(|c| c.ticket_name.clone().unwrap_or("".to_string()))
                    .unwrap_or("".to_string());
                data.push(TicketCountReportRow {
                    row_name,
                    daily_sold: ticket_type_counts
                        .iter()
                        .map(|c| c.purchased_yesterday_count)
                        .sum::<i64>()
                        - ticket_type_counts
                            .iter()
                            .map(|c| c.comp_purchased_yesterday_count)
                            .sum::<i64>(),
                    total_sold: ticket_type_counts.iter().map(|c| c.purchased_count).sum::<i64>()
                        - ticket_type_counts.iter().map(|c| c.comp_purchased_count).sum::<i64>(),
                    held: ticket_type_counts.iter().map(|c| c.hold_available_count).sum::<i64>()
                        + ticket_type_counts.iter().map(|c| c.comp_available_count).sum::<i64>(),
                    open: ticket_type_counts.iter().map(|c| c.available_for_purchase_count).sum(),
                    total_value_in_cents: *total_sales.get(&ticket_type_id).unwrap_or(&0),
                });
            }
        }

        data.push(TicketCountReportRow {
            row_name: "Totals".to_string(),
            daily_sold: data.iter().map(|d| d.daily_sold).sum(),
            total_sold: data.iter().map(|d| d.total_sold).sum(),
            held: data.iter().map(|d| d.held).sum::<i64>(),
            open: data.iter().map(|d| d.open).sum(),
            total_value_in_cents: data.iter().map(|d| d.total_value_in_cents).sum(),
        });

        TicketCountReport { data }
    }
}
