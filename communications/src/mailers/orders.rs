use crate::config::Config;
use crate::errors::*;
use bigneon_db::models::*;
use bigneon_db::prelude::{DisplayOrder, OrderItem, Refund};
use diesel::PgConnection;
use itertools::Itertools;

pub fn confirmation_email(
    user_first_name: &String,
    user_email: String,
    display_order: DisplayOrder,
    config: &Config,
    conn: &PgConnection,
) -> Result<Communication, BigNeonError> {
    let source = CommAddress::from(config.communication_default_source_email.clone());
    let destinations = CommAddress::from(user_email);
    let title = "BigNeon Purchase Completed".to_string();
    let template_id = config.sendgrid_template_bn_purchase_completed.clone();
    let mut template_data = TemplateData::new();
    template_data.insert(String::from("name"), user_first_name.clone());
    //Construct an itemised breakdown using a HTML table
    let mut item_breakdown = r#"<table style="width:100%"><tbody>"#.to_string();
    item_breakdown.push_str("<tr><th>Units</th><th>Description</th><th>Unit Price</th><th>Total</th></tr>");
    let mut total_fees = 0;
    let mut total_initial_fees = 0;
    let mut total_refunded_fees = 0;

    for oi in &display_order.items {
        match oi.item_type {
            OrderItemTypes::Tickets => {
                item_breakdown.push_str(&generate_item_row(
                    &oi.description,
                    oi.quantity,
                    oi.unit_price_in_cents,
                    false,
                ));
                let mut discount_per_ticket = 0;

                if let Some(discount_item) = OrderItem::find(oi.id, conn)?.find_discount_item(conn)? {
                    discount_per_ticket = discount_item.unit_price_in_cents;
                    item_breakdown.push_str(&generate_item_row(
                        "Discount",
                        discount_item.quantity,
                        discount_item.unit_price_in_cents,
                        false,
                    ));
                }

                if oi.refunded_quantity > 0 {
                    item_breakdown.push_str(&generate_item_row(
                        "Refunded",
                        oi.refunded_quantity,
                        oi.unit_price_in_cents + discount_per_ticket,
                        true,
                    ));
                }
            }
            // Do nothing, included above with ticket for display
            OrderItemTypes::Discount => (),
            _ => {
                //Accumulate fees
                total_initial_fees += oi.quantity * oi.unit_price_in_cents;
                total_refunded_fees += oi.refunded_quantity * oi.unit_price_in_cents;
                total_fees += (oi.quantity - oi.refunded_quantity) * oi.unit_price_in_cents;
            }
        }
    }
    item_breakdown.push_str("</tbody></table>");

    let mut total_breakdown = r#"<table><tbody>"#.to_string();
    if total_initial_fees > 0 {
        total_breakdown.push_str(&format!(
            "<tr><th>Fees Total</th><td>{}</td></tr>",
            format!("{:.*}", 2, total_initial_fees as f64 / 100.0)
        ));
    }
    if total_refunded_fees > 0 {
        total_breakdown.push_str(&format!(
            r#"<tr style="color: red"><th>Refunded</th><td>(${})</td></tr>"#,
            format!("{:.*}", 2, total_refunded_fees as f64 / 100.0)
        ));
    }
    total_breakdown.push_str(&format!(
        "<tr><th>Order Total</th><td>{}</td></tr>",
        format!("{:.*}", 2, display_order.total_in_cents as f64 / 100.0)
    ));
    total_breakdown.push_str("</tbody></table>");

    template_data.insert(
        "ticket_count".to_string(),
        display_order
            .items
            .iter()
            .filter(|i| i.item_type == OrderItemTypes::Tickets)
            .map(|i| i.quantity - i.refunded_quantity)
            .sum::<i64>()
            .to_string(),
    );

    template_data.insert(
        "total_initial_fees".to_string(),
        format!("{:.*}", 2, total_initial_fees as f64 / 100.0),
    );
    template_data.insert(
        "total_refunded_fees".to_string(),
        format!("{:.*}", 2, total_refunded_fees as f64 / 100.0),
    );
    template_data.insert("total_fees".to_string(), format!("{:.*}", 2, total_fees as f64 / 100.0));
    template_data.insert(
        "total_price".to_string(),
        format!("{:.*}", 2, display_order.total_in_cents as f64 / 100.0),
    );
    template_data.insert("item_breakdown".to_string(), item_breakdown);
    template_data.insert("total_breakdown".to_string(), total_breakdown);
    template_data.insert("tickets_link".to_string(), format!("{}/hub", config.front_end_url));

    // TODO: Perhaps move this to an event subscription
    Ok(Communication::new(
        CommunicationType::EmailTemplate,
        title,
        None,
        Some(source),
        destinations,
        Some(template_id),
        Some(vec![template_data]),
        Some(vec!["purchase".to_string()]),
        None,
    ))
}

fn generate_item_row(description: &str, quantity: i64, unit_price_in_cents: i64, refund: bool) -> String {
    let mut item_row = "".to_string();

    if refund {
        item_row.push_str(r#"<tr style="color: red">"#);
    } else {
        item_row.push_str("<tr>");
    }
    item_row.push_str(r#"<td align="center">"#);
    item_row.push_str(&quantity.to_string());
    item_row.push_str("</td><td>");
    item_row.push_str(&description);
    item_row.push_str(r#"</td><td align="right">"#);

    let mut unit_price_display = format!("${:.*}", 2, unit_price_in_cents.abs() as f64 / 100.0);
    if unit_price_in_cents < 0 || refund {
        unit_price_display = format!("({})", unit_price_display);
    }
    item_row.push_str(&unit_price_display);
    item_row.push_str(r#"</td><td align="right">"#);

    let mut total_price_display = format!("${:.*}", 2, (quantity * unit_price_in_cents.abs()) as f64 / 100.0);
    if unit_price_in_cents < 0 || refund {
        total_price_display = format!("({})", total_price_display);
    }
    item_row.push_str(&total_price_display);
    item_row.push_str("</td></tr>");

    item_row
}

pub fn refund_email(
    user_first_name: &String,
    user_email: String,
    refund: &Refund,
    config: &Config,
    conn: &PgConnection,
) -> Result<(), BigNeonError> {
    let source = CommAddress::from(config.communication_default_source_email.clone());
    let destinations = CommAddress::from(user_email);
    let title = "BigNeon Refund".to_string();
    let template_id = config.sendgrid_template_bn_refund.clone();
    let mut template_data = TemplateData::new();
    template_data.insert(String::from("name"), user_first_name.clone());
    //Construct an itemised breakdown using a HTML table
    let mut item_breakdown = r#"<table style="width:100%"><tbody>"#.to_string();
    item_breakdown.push_str("<tr><th>Units Refunded</th><th>Description</th><th>Total</th></tr>");

    let items = refund
        .items(conn)?
        .into_iter()
        .map(|i| {
            let item = OrderItem::find(i.order_item_id, conn);
            return (i, item);
        })
        .collect_vec();

    let mut new_items = vec![];
    // unwrap results
    for (item, res) in items {
        new_items.push((item, res?));
    }
    let items = new_items;

    for (item, oi) in items.iter() {
        item_breakdown.push_str(r#"<tr><th align="center">"#);
        item_breakdown.push_str(&item.quantity.to_string());
        item_breakdown.push_str("</th><th>");
        item_breakdown.push_str(&oi.description(conn)?);
        item_breakdown.push_str(r#"</th><th align="right">$"#);
        item_breakdown.push_str(&format!("{:.*}", 2, item.amount as f64 / 100.0));

        item_breakdown.push_str("</th></tr>");
    }

    item_breakdown.push_str("</tbody></table>");
    let amount = items.iter().map(|i| i.0.amount).sum::<i64>() as f64 / 100.0;
    template_data.insert("amount_refunded".to_string(), amount.to_string());
    template_data.insert(
        "ticket_count".to_string(),
        items
            .iter()
            .filter(|i| i.1.item_type == OrderItemTypes::Tickets)
            .map(|i| i.0.quantity)
            .sum::<i64>()
            .to_string(),
    );
    let total_fees = items
        .iter()
        .filter(|i| i.1.item_type.is_fee())
        .map(|i| i.0.amount)
        .sum::<i64>();

    template_data.insert("total_fees".to_string(), format!("{:.*}", 2, total_fees as f64 / 100.0));
    template_data.insert("total_price".to_string(), format!("{:.*}", 2, amount));
    template_data.insert("item_breakdown".to_string(), item_breakdown);
    template_data.insert("tickets_link".to_string(), format!("{}/orders", config.front_end_url));

    // TODO: Perhaps move this to an event subscription
    Communication::new(
        CommunicationType::EmailTemplate,
        title,
        None,
        Some(source),
        destinations,
        Some(template_id),
        Some(vec![template_data]),
        Some(vec!["refund"]),
        None,
    )
    .queue(conn)?;

    Ok(())
}
