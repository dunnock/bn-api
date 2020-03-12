use crate::support;
use crate::support::database::TestDatabase;
use crate::support::test_request::TestRequest;
use actix_web::{http::StatusCode, HttpResponse, web::{Path, Query}};
use bigneon_api::controllers::reports::{self, *};
use bigneon_api::errors::BigNeonError;
use bigneon_api::models::{PathParameters, WebPayload};
use bigneon_db::models::*;
use bigneon_db::schema::orders;
use chrono::prelude::*;
use chrono::Duration;
use diesel;
use diesel::prelude::*;
use serde_json;

pub fn box_office_sales_summary(role: Roles, should_succeed: bool) {
    let database = TestDatabase::new();
    let box_office_user = database.create_user().with_first_name("BoxOfficeUser1").finish();
    let box_office_user2 = database.create_user().with_first_name("BoxOfficeUser2").finish();
    let user = database.create_user().finish();
    let user2 = database.create_user().finish();
    let user3 = database.create_user().finish();
    let organization = database
        .create_organization()
        .with_member(&box_office_user, Roles::OrgBoxOffice)
        .with_member(&box_office_user2, Roles::OrgBoxOffice)
        .with_fees()
        .finish();
    let event = database
        .create_event()
        .with_organization(&organization)
        .with_event_start(NaiveDate::from_ymd(2016, 7, 8).and_hms(9, 10, 11))
        .with_name("Event1".to_string())
        .with_tickets()
        .with_ticket_pricing()
        .finish();
    let event2 = database
        .create_event()
        .with_organization(&organization)
        .with_event_start(NaiveDate::from_ymd(2017, 7, 8).and_hms(9, 10, 11))
        .with_name("Event2".to_string())
        .with_tickets()
        .with_ticket_pricing()
        .finish();

    let mut box_office_user_orders = Vec::new();
    let mut box_office_user2_orders = Vec::new();
    box_office_user_orders.push(
        database
            .create_order()
            .quantity(2)
            .for_event(&event)
            .for_user(&box_office_user)
            .on_behalf_of_user(&user)
            .is_paid()
            .finish(),
    );
    box_office_user_orders.push(
        database
            .create_order()
            .quantity(1)
            .for_event(&event2)
            .for_user(&box_office_user)
            .on_behalf_of_user(&user)
            .is_paid()
            .finish(),
    );
    box_office_user_orders.push(
        database
            .create_order()
            .quantity(1)
            .for_event(&event2)
            .for_user(&box_office_user)
            .on_behalf_of_user(&user3)
            .is_paid()
            .finish(),
    );
    box_office_user2_orders.push(
        database
            .create_order()
            .quantity(2)
            .for_event(&event)
            .for_user(&box_office_user2)
            .on_behalf_of_user(&user2)
            .is_paid()
            .finish(),
    );

    let auth_db_user = database.create_user().finish();
    let auth_user = support::create_auth_user_from_user(&auth_db_user, role, Some(&organization), &database);

    let test_request = TestRequest::create_with_uri("/reports?report=box_office_sales_summary");
    let mut path = Path::<PathParameters>::extract(&test_request.request).unwrap();
    path.id = organization.id;
    let query = Query::<ReportQueryParameters>::extract(&test_request.request).unwrap();
    let response: HttpResponse =
        reports::box_office_sales_summary((database.connection.clone().into(), query, path, auth_user)).into();

    if !should_succeed {
        support::expects_unauthorized(&response);
        return;
    }

    assert_eq!(response.status(), StatusCode::OK);
    let body = support::unwrap_body_to_string(&response).unwrap();
    let report_data: BoxOfficeSalesSummaryReport = serde_json::from_str(&body).unwrap();

    let expected_report_data = BoxOfficeSalesSummaryReport {
        operators: vec![
            BoxOfficeSalesSummaryOperatorRow {
                operator_id: box_office_user.id,
                operator_name: box_office_user.full_name(),
                events: vec![
                    BoxOfficeSalesSummaryOperatorEventRow {
                        event_name: Some("Event1".to_string()),
                        event_date: Some(NaiveDate::from_ymd(2016, 7, 8).and_hms(9, 10, 11)),
                        number_of_tickets: 2,
                        face_value_in_cents: 150,
                        revenue_share_value_in_cents: 0,
                        total_sales_in_cents: 300,
                    },
                    BoxOfficeSalesSummaryOperatorEventRow {
                        event_name: Some("Event2".to_string()),
                        event_date: Some(NaiveDate::from_ymd(2017, 7, 8).and_hms(9, 10, 11)),
                        number_of_tickets: 2,
                        face_value_in_cents: 150,
                        revenue_share_value_in_cents: 0,
                        total_sales_in_cents: 300,
                    },
                ],
                payments: vec![BoxOfficeSalesSummaryPaymentRow {
                    payment_type: ExternalPaymentType::CreditCard,
                    quantity: 4,
                    total_sales_in_cents: 600,
                }],
            },
            BoxOfficeSalesSummaryOperatorRow {
                operator_id: box_office_user2.id,
                operator_name: box_office_user2.full_name(),
                events: vec![BoxOfficeSalesSummaryOperatorEventRow {
                    event_name: Some("Event1".to_string()),
                    event_date: Some(NaiveDate::from_ymd(2016, 7, 8).and_hms(9, 10, 11)),
                    number_of_tickets: 2,
                    face_value_in_cents: 150,
                    revenue_share_value_in_cents: 0,
                    total_sales_in_cents: 300,
                }],
                payments: vec![BoxOfficeSalesSummaryPaymentRow {
                    payment_type: ExternalPaymentType::CreditCard,
                    quantity: 2,
                    total_sales_in_cents: 300,
                }],
            },
        ],
        payments: vec![BoxOfficeSalesSummaryPaymentRow {
            payment_type: ExternalPaymentType::CreditCard,
            quantity: 6,
            total_sales_in_cents: 900,
        }],
    };

    assert_eq!(expected_report_data, report_data);
}

pub fn transaction_detail_report(role: Roles, should_succeed: bool, filter_event: bool) {
    let database = TestDatabase::new();
    let connection = database.connection.get();
    let organization = database.create_organization().with_event_fee().with_fees().finish();
    let event = database
        .create_event()
        .with_organization(&organization)
        .with_name("Event1".to_string())
        .with_tickets()
        .with_ticket_pricing()
        .finish();
    let ticket_type = event.ticket_types(true, None, connection).unwrap().remove(0);
    let event2 = database
        .create_event()
        .with_organization(&organization)
        .with_name("Event2".to_string())
        .with_tickets()
        .with_ticket_pricing()
        .finish();
    let ticket_type2 = event2.ticket_types(true, None, connection).unwrap().remove(0);

    let fee_schedule = FeeSchedule::find(organization.fee_schedule_id, connection).unwrap();
    let ticket_pricing = ticket_type.current_ticket_pricing(false, connection).unwrap();
    let fee_schedule_range = fee_schedule
        .get_range(ticket_pricing.price_in_cents, connection)
        .unwrap();
    let ticket_pricing2 = ticket_type2.current_ticket_pricing(false, connection).unwrap();
    let fee_schedule_range2 = fee_schedule
        .get_range(ticket_pricing2.price_in_cents, connection)
        .unwrap();

    let user = database.create_user().finish();
    let user2 = database.create_user().finish();
    let user3 = database.create_user().finish();
    let user4 = database.create_user().finish();

    let mut order = database
        .create_order()
        .quantity(2)
        .for_event(&event)
        .for_user(&user)
        .is_paid()
        .with_external_payment_type(ExternalPaymentType::Voucher)
        .finish();
    let order_paid_at = Utc::now().naive_utc() - Duration::days(5);
    order = diesel::update(orders::table.filter(orders::id.eq(order.id)))
        .set(orders::paid_at.eq(order_paid_at))
        .get_result::<Order>(connection)
        .unwrap();

    let mut order2 = database
        .create_order()
        .quantity(2)
        .for_event(&event)
        .for_user(&user2)
        .is_paid()
        .with_external_payment_type(ExternalPaymentType::Voucher)
        .finish();
    let order2_paid_at = Utc::now().naive_utc() - Duration::days(4);
    order2 = diesel::update(orders::table.filter(orders::id.eq(order2.id)))
        .set(orders::paid_at.eq(order2_paid_at))
        .get_result::<Order>(connection)
        .unwrap();

    let mut order3 = database
        .create_order()
        .quantity(2)
        .for_event(&event2)
        .for_user(&user3)
        .is_paid()
        .with_external_payment_type(ExternalPaymentType::Voucher)
        .finish();
    let order3_paid_at = Utc::now().naive_utc() - Duration::days(3);
    order3 = diesel::update(orders::table.filter(orders::id.eq(order3.id)))
        .set(orders::paid_at.eq(order3_paid_at))
        .get_result::<Order>(connection)
        .unwrap();

    let mut order4 = database
        .create_order()
        .quantity(2)
        .for_event(&event)
        .for_user(&user4)
        .is_paid()
        .with_external_payment_type(ExternalPaymentType::Voucher)
        .finish();
    let order4_paid_at = Utc::now().naive_utc() - Duration::days(2);
    order4 = diesel::update(orders::table.filter(orders::id.eq(order4.id)))
        .set(orders::paid_at.eq(order4_paid_at))
        .get_result::<Order>(connection)
        .unwrap();

    let auth_db_user = database.create_user().finish();
    let auth_user = support::create_auth_user_from_user(&auth_db_user, role, Some(&organization), &database);

    let event_filter = format!("/reports?report=transaction_detail_report&event_id={}", event.id);
    let test_request = if filter_event {
        TestRequest::create_with_uri(&event_filter)
    } else {
        TestRequest::create_with_uri("/reports?report=transaction_detail_report")
    };
    let mut path = Path::<PathParameters>::extract(&test_request.request).unwrap();
    path.id = organization.id;
    let query = Query::<ReportQueryParameters>::extract(&test_request.request).unwrap();
    let response: Result<WebPayload<TransactionReportRow>, BigNeonError> =
        reports::transaction_detail_report((database.connection.clone().into(), query, path, auth_user));

    if !should_succeed {
        support::expects_unauthorized(&response.unwrap_err().into_inner().to_response());
        return;
    }

    let response = response.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let report_payload = response.payload();

    let paging = Paging::new(0, 100);
    let mut payload = Payload::new(
        if filter_event {
            vec![
                build_transaction_report_row(
                    3,
                    &organization,
                    &user4,
                    &order4,
                    &event,
                    &ticket_type,
                    &fee_schedule_range,
                    2,
                    ticket_pricing.price_in_cents,
                ),
                build_transaction_report_row(
                    3,
                    &organization,
                    &user2,
                    &order2,
                    &event,
                    &ticket_type,
                    &fee_schedule_range,
                    2,
                    ticket_pricing.price_in_cents,
                ),
                build_transaction_report_row(
                    3,
                    &organization,
                    &user,
                    &order,
                    &event,
                    &ticket_type,
                    &fee_schedule_range,
                    2,
                    ticket_pricing.price_in_cents,
                ),
            ]
        } else {
            vec![
                build_transaction_report_row(
                    4,
                    &organization,
                    &user4,
                    &order4,
                    &event,
                    &ticket_type,
                    &fee_schedule_range,
                    2,
                    ticket_pricing.price_in_cents,
                ),
                build_transaction_report_row(
                    4,
                    &organization,
                    &user3,
                    &order3,
                    &event2,
                    &ticket_type2,
                    &fee_schedule_range2,
                    2,
                    ticket_pricing2.price_in_cents,
                ),
                build_transaction_report_row(
                    4,
                    &organization,
                    &user2,
                    &order2,
                    &event,
                    &ticket_type,
                    &fee_schedule_range,
                    2,
                    ticket_pricing.price_in_cents,
                ),
                build_transaction_report_row(
                    4,
                    &organization,
                    &user,
                    &order,
                    &event,
                    &ticket_type,
                    &fee_schedule_range,
                    2,
                    ticket_pricing.price_in_cents,
                ),
            ]
        },
        paging,
    );

    payload.paging.total = if filter_event { 3 } else { 4 };
    payload.paging.dir = SortingDir::Asc;

    assert_eq!(report_payload, &payload);
}

fn build_transaction_report_row(
    total: i64,
    organization: &Organization,
    user: &User,
    order: &Order,
    event: &Event,
    ticket_type: &TicketType,
    fee_schedule_range: &FeeScheduleRange,
    quantity: i64,
    price_per_ticket: i64,
) -> TransactionReportRow {
    TransactionReportRow {
        total,
        quantity,
        event_name: event.name.clone(),
        ticket_name: ticket_type.name.clone(),
        actual_quantity: quantity,
        refunded_quantity: 0,
        unit_price_in_cents: price_per_ticket,
        face_price_in_cents: price_per_ticket,
        face_price_in_cents_total: price_per_ticket * quantity,
        gross: (price_per_ticket + fee_schedule_range.client_fee_in_cents) * quantity,
        client_fee_in_cents: fee_schedule_range.client_fee_in_cents,
        client_fee_in_cents_total: fee_schedule_range.client_fee_in_cents * quantity,
        event_fee_client_in_cents: organization.client_event_fee_in_cents,
        event_fee_client_in_cents_total: organization.client_event_fee_in_cents,
        fee_range_id: Some(fee_schedule_range.id),
        order_type: OrderTypes::Cart,
        payment_method: Some(PaymentMethods::CreditCard.to_string()),
        payment_provider: Some(PaymentProviders::Stripe.to_string()),
        transaction_date: order.paid_at.clone().unwrap(),
        redemption_code: None,
        order_id: order.id,
        event_id: event.id,
        user_id: user.id,
        first_name: user.first_name.clone().unwrap(),
        last_name: user.last_name.clone().unwrap(),
        email: user.email.clone().unwrap(),
        event_start: event.event_start,
        promo_discount_value_in_cents: 0,
        promo_quantity: 0,
        promo_code_name: None,
        promo_redemption_code: None,
        source: None,
        medium: None,
        campaign: None,
        term: None,
        content: None,
        platform: None,
        check_in_source: None,
    }
}
