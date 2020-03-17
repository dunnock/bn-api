use crate::support;
use crate::support::database::TestDatabase;
use crate::support::test_request::TestRequest;
use actix_web::{http::StatusCode, web::Query, FromRequest};
use bigneon_api::controllers::admin::reports::{self, *};
use bigneon_api::errors::BigNeonError;
use bigneon_api::models::WebPayload;
use bigneon_db::models::*;
use bigneon_db::schema::orders;
use chrono::prelude::*;
use chrono::Duration;
use diesel;
use diesel::prelude::*;

pub async fn domain_transaction_detail_report(role: Roles, should_succeed: bool) {
    let database = TestDatabase::new();
    let connection = database.connection.get();
    let organization = database
        .create_organization()
        .with_event_fee()
        .with_fees()
        .with_cc_fee(1.1)
        .finish();
    let event = database
        .create_event()
        .with_organization(&organization)
        .with_name("Event1".to_string())
        .with_tickets()
        .with_ticket_pricing()
        .with_event_start(Utc::now().naive_utc() + Duration::days(20))
        .finish();
    let ticket_type = event.ticket_types(true, None, connection).unwrap().remove(0);

    let user = database
        .create_user()
        .with_first_name("Bob".into())
        .with_last_name("Bobber".into())
        .with_email("bobber@tari.com".into())
        .finish();
    let user2 = database
        .create_user()
        .with_first_name("Bobby".into())
        .with_last_name("Last".into())
        .with_email("bobby.last@tari.com".into())
        .finish();

    let mut order = database
        .create_order()
        .quantity(2)
        .for_event(&event)
        .for_user(&user)
        .is_paid()
        .finish();
    let order_paid_at = Utc::now().naive_utc() - Duration::days(6);
    order = diesel::update(orders::table.filter(orders::id.eq(order.id)))
        .set(orders::paid_at.eq(order_paid_at))
        .get_result::<Order>(connection)
        .unwrap();

    let order2 = database
        .create_order()
        .quantity(1)
        .for_event(&event)
        .for_user(&user2)
        .is_paid()
        .finish();
    let order2_paid_at = Utc::now().naive_utc() - Duration::days(4);
    diesel::update(orders::table.filter(orders::id.eq(order2.id)))
        .set(orders::paid_at.eq(order2_paid_at))
        .get_result::<Order>(connection)
        .unwrap();

    let auth_db_user = database.create_user().finish();
    let auth_user = support::create_auth_user_from_user(&auth_db_user, role, Some(&organization), &database);

    let fmt_string = "%Y-%m-%dT%H:%M:%S";
    let query = format!("/admin/reports?name=domain_transaction_detail?transaction_start_utc={}&transaction_end_utc={}&event_start_utc={}&event_end_utc={}&page={}&limit={}",
        (order.paid_at.unwrap() - Duration::hours(1)).format(fmt_string),
        (order.paid_at.unwrap() + Duration::hours(1)).format(fmt_string),
        (event.event_start.unwrap() - Duration::hours(1)).format(fmt_string),
        (event.event_start.unwrap() + Duration::hours(1)).format(fmt_string),
        0,
        1,
    );

    let test_request = TestRequest::create_with_uri(&query);

    let query = Query::<ReportQueryParameters>::extract(&test_request.request)
        .await
        .unwrap();
    let response: Result<WebPayload<DomainTransactionReportRow>, BigNeonError> =
        reports::domain_transaction_detail_report((database.connection.clone().into(), query, auth_user));

    if !should_succeed {
        support::expects_unauthorized(&response.unwrap_err().into_inner().to_response());
        return;
    }

    let response = response.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let report_payload = response.payload();

    let paging = Paging::new(0, 100);
    let mut payload = Payload::new(
        vec![DomainTransactionReportRow {
            total: 1,
            order_id: order.id,
            customer_name_first: user.first_name.clone(),
            customer_name_last: user.last_name.clone(),
            customer_email_address: user.email.clone(),
            event_name: event.name.clone(),
            event_date: event.event_start.clone(),
            ticket_type_name: ticket_type.name.clone(),
            transaction_date: order.paid_at.unwrap(),
            point_of_sale: None,
            payment_method: PaymentMethods::CreditCard.to_string(),
            qty_tickets_sold: 2,
            qty_tickets_refunded: 0,
            qty_tickets_sold_net: 2,
            face_price_in_cents: 150,
            total_face_value_in_cents: 300,
            client_per_ticket_revenue_in_cents: 162,
            client_per_order_revenue_in_cents: 174,
            company_per_ticket_revenue_in_cents: 108,
            company_per_order_revenue_in_cents: 116,
            credit_card_processing_fees_in_cents: 6,
            gross: 596,
        }],
        paging,
    );

    payload.paging.total = 1;
    payload.paging.limit = 1;

    assert_eq!(report_payload, &payload);
}
