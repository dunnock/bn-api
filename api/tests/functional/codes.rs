use crate::functional::base;
use crate::support;
use crate::support::database::TestDatabase;
use crate::support::test_request::TestRequest;
use actix_web::{http::StatusCode, web::Path, FromRequest, HttpResponse};
use bigneon_api::controllers::codes::{self, *};
use bigneon_api::extractors::*;
use bigneon_api::models::PathParameters;
use bigneon_db::models::*;
use chrono::prelude::*;
use chrono::Duration;
use chrono::NaiveDateTime;

#[cfg(test)]
mod show_tests {
    use super::*;
    #[actix_rt::test]
    async fn show_org_member() {
        base::codes::show(Roles::OrgMember, true).await;
    }
    #[actix_rt::test]
    async fn show_admin() {
        base::codes::show(Roles::Admin, true).await;
    }
    #[actix_rt::test]
    async fn show_user() {
        base::codes::show(Roles::User, false).await;
    }
    #[actix_rt::test]
    async fn show_org_owner() {
        base::codes::show(Roles::OrgOwner, true).await;
    }
    #[actix_rt::test]
    async fn show_door_person() {
        base::codes::show(Roles::DoorPerson, true).await;
    }
    #[actix_rt::test]
    async fn show_promoter() {
        base::codes::show(Roles::Promoter, true).await;
    }
    #[actix_rt::test]
    async fn show_promoter_read_only() {
        base::codes::show(Roles::PromoterReadOnly, true).await;
    }
    #[actix_rt::test]
    async fn show_org_admin() {
        base::codes::show(Roles::OrgAdmin, true).await;
    }
    #[actix_rt::test]
    async fn show_box_office() {
        base::codes::show(Roles::OrgBoxOffice, true).await;
    }
}

#[cfg(test)]
mod create_tests {
    use super::*;
    #[actix_rt::test]
    async fn create_org_member() {
        base::codes::create(Roles::OrgMember, true).await;
    }
    #[actix_rt::test]
    async fn create_admin() {
        base::codes::create(Roles::Admin, true).await;
    }
    #[actix_rt::test]
    async fn create_user() {
        base::codes::create(Roles::User, false).await;
    }
    #[actix_rt::test]
    async fn create_org_owner() {
        base::codes::create(Roles::OrgOwner, true).await;
    }
    #[actix_rt::test]
    async fn create_door_person() {
        base::codes::create(Roles::DoorPerson, false).await;
    }
    #[actix_rt::test]
    async fn create_promoter() {
        base::codes::create(Roles::Promoter, true).await;
    }
    #[actix_rt::test]
    async fn create_promoter_read_only() {
        base::codes::create(Roles::PromoterReadOnly, false).await;
    }
    #[actix_rt::test]
    async fn create_org_admin() {
        base::codes::create(Roles::OrgAdmin, true).await;
    }
    #[actix_rt::test]
    async fn create_box_office() {
        base::codes::create(Roles::OrgBoxOffice, false).await;
    }
}

#[cfg(test)]
mod update_tests {
    use super::*;
    #[actix_rt::test]
    async fn update_org_member() {
        base::codes::update(Roles::OrgMember, true).await;
    }
    #[actix_rt::test]
    async fn update_admin() {
        base::codes::update(Roles::Admin, true).await;
    }
    #[actix_rt::test]
    async fn update_user() {
        base::codes::update(Roles::User, false).await;
    }
    #[actix_rt::test]
    async fn update_org_owner() {
        base::codes::update(Roles::OrgOwner, true).await;
    }
    #[actix_rt::test]
    async fn update_door_person() {
        base::codes::update(Roles::DoorPerson, false).await;
    }
    #[actix_rt::test]
    async fn update_promoter() {
        base::codes::update(Roles::Promoter, true).await;
    }
    #[actix_rt::test]
    async fn update_promoter_read_only() {
        base::codes::update(Roles::PromoterReadOnly, false).await;
    }
    #[actix_rt::test]
    async fn update_org_admin() {
        base::codes::update(Roles::OrgAdmin, true).await;
    }
    #[actix_rt::test]
    async fn update_box_office() {
        base::codes::update(Roles::OrgBoxOffice, false).await;
    }
}

#[cfg(test)]
mod destroy_tests {
    use super::*;
    #[actix_rt::test]
    async fn destroy_org_member() {
        base::codes::destroy(Roles::OrgMember, true).await;
    }
    #[actix_rt::test]
    async fn destroy_admin() {
        base::codes::destroy(Roles::Admin, true).await;
    }
    #[actix_rt::test]
    async fn destroy_user() {
        base::codes::destroy(Roles::User, false).await;
    }
    #[actix_rt::test]
    async fn destroy_org_owner() {
        base::codes::destroy(Roles::OrgOwner, true).await;
    }
    #[actix_rt::test]
    async fn destroy_door_person() {
        base::codes::destroy(Roles::DoorPerson, false).await;
    }
    #[actix_rt::test]
    async fn destroy_promoter() {
        base::codes::destroy(Roles::Promoter, true).await;
    }
    #[actix_rt::test]
    async fn destroy_promoter_read_only() {
        base::codes::destroy(Roles::PromoterReadOnly, false).await;
    }
    #[actix_rt::test]
    async fn destroy_org_admin() {
        base::codes::destroy(Roles::OrgAdmin, true).await;
    }
    #[actix_rt::test]
    async fn destroy_box_office() {
        base::codes::destroy(Roles::OrgBoxOffice, false).await;
    }
}

#[actix_rt::test]
async fn create_with_validation_errors() {
    let database = TestDatabase::new();
    let connection = database.connection.get();
    let user = database.create_user().finish();
    let event = database.create_event().with_ticket_pricing().finish();
    let ticket_type_id = event.ticket_types(true, None, connection).unwrap()[0].id;
    let organization = event.organization(connection).unwrap();
    let auth_user = support::create_auth_user_from_user(&user, Roles::OrgOwner, Some(&organization), &database);

    let start_date = NaiveDateTime::from(Utc::now().naive_utc() + Duration::days(1));
    let end_date = NaiveDateTime::from(Utc::now().naive_utc() - Duration::days(2));
    let json = Json(CreateCodeRequest {
        name: "Code Example".into(),
        redemption_codes: vec!["a".into()],
        code_type: CodeTypes::Discount,
        max_uses: 10,
        discount_in_cents: Some(100),
        discount_as_percentage: None,
        start_date: Some(start_date),
        end_date: Some(end_date),
        max_tickets_per_user: None,
        ticket_type_ids: vec![ticket_type_id],
    });

    let test_request = TestRequest::create();
    let mut path = Path::<PathParameters>::extract(&test_request.request).await.unwrap();
    path.id = event.id;

    let response: HttpResponse = codes::create((database.connection.clone().into(), json, path, auth_user.clone()))
        .await
        .into();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert!(response.error().is_some());

    let validation_response = support::validation_response_from_response(&response).unwrap();
    let start_date_err = validation_response.fields.get("start_date").unwrap();
    assert_eq!(start_date_err[0].code, "start_date_must_be_before_end_date");
    assert_eq!(
        &start_date_err[0].message.clone().unwrap().into_owned(),
        "Start date must be before end date"
    );

    let json = Json(CreateCodeRequest {
        name: "Code Example".into(),
        redemption_codes: vec!["a".into()],
        code_type: CodeTypes::Discount,
        max_uses: 10,
        discount_in_cents: None,
        discount_as_percentage: None,
        start_date: Some(start_date),
        end_date: Some(end_date),
        max_tickets_per_user: None,
        ticket_type_ids: vec![ticket_type_id],
    });

    let test_request = TestRequest::create();
    let mut path = Path::<PathParameters>::extract(&test_request.request).await.unwrap();
    path.id = event.id;

    let response: HttpResponse = codes::create((database.connection.clone().into(), json, path, auth_user.clone()))
        .await
        .into();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert!(response.error().is_some());

    let validation_response = support::validation_response_from_response(&response).unwrap();
    let discount_err = validation_response.fields.get("discounts").unwrap();
    assert_eq!(discount_err[0].code, "required");

    let json = Json(CreateCodeRequest {
        name: "Code Example".into(),
        redemption_codes: vec!["a".into()],
        code_type: CodeTypes::Discount,
        max_uses: 10,
        discount_in_cents: Some(100),
        discount_as_percentage: Some(15),
        start_date: Some(start_date),
        end_date: Some(end_date),
        max_tickets_per_user: None,
        ticket_type_ids: vec![ticket_type_id],
    });

    let test_request = TestRequest::create();
    let mut path = Path::<PathParameters>::extract(&test_request.request).await.unwrap();
    path.id = event.id;

    let response: HttpResponse = codes::create((database.connection.clone().into(), json, path, auth_user))
        .await
        .into();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert!(response.error().is_some());

    let validation_response = support::validation_response_from_response(&response).unwrap();
    let discount_err = validation_response.fields.get("discounts").unwrap();
    assert_eq!(discount_err[0].code, "only_single_discount_type_allowed");
}

#[actix_rt::test]
async fn create_fails_adding_ticket_type_id_from_other_event() {
    let database = TestDatabase::new();
    let connection = database.connection.get();
    let user = database.create_user().finish();
    let event = database.create_event().with_ticket_pricing().finish();
    let event2 = database.create_event().with_ticket_pricing().finish();
    let ticket_type_id = event2.ticket_types(true, None, connection).unwrap()[0].id;
    let organization = event.organization(connection).unwrap();
    let auth_user = support::create_auth_user_from_user(&user, Roles::OrgOwner, Some(&organization), &database);

    let start_date = NaiveDateTime::from(Utc::now().naive_utc() - Duration::days(1));
    let end_date = NaiveDateTime::from(Utc::now().naive_utc() + Duration::days(2));
    let json = Json(CreateCodeRequest {
        name: "Code Example".into(),
        redemption_codes: vec!["REDEMPTIONCODE".into()],
        code_type: CodeTypes::Discount,
        max_uses: 10,
        discount_in_cents: Some(100),
        discount_as_percentage: None,
        start_date: Some(start_date),
        end_date: Some(end_date),
        max_tickets_per_user: None,
        ticket_type_ids: vec![ticket_type_id],
    });

    let test_request = TestRequest::create();
    let mut path = Path::<PathParameters>::extract(&test_request.request).await.unwrap();
    path.id = event.id;

    let response: HttpResponse = codes::create((database.connection.clone().into(), json, path, auth_user))
        .await
        .into();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert!(response.error().is_some());

    let validation_response = support::validation_response_from_response(&response).unwrap();
    let ticket_type_id = validation_response.fields.get("ticket_type_id").unwrap();
    assert_eq!(ticket_type_id[0].code, "invalid");
    assert_eq!(
        &ticket_type_id[0].message.clone().unwrap().into_owned(),
        "Ticket type not valid for code as it does not belong to same event"
    );
}

#[actix_rt::test]
async fn update_with_validation_errors() {
    let database = TestDatabase::new();
    let connection = database.connection.get();
    let user = database.create_user().finish();
    let event = database.create_event().finish();
    let code = database.create_code().with_event(&event).finish();
    let organization = event.organization(connection).unwrap();
    let auth_user = support::create_auth_user_from_user(&user, Roles::OrgOwner, Some(&organization), &database);

    let test_request = TestRequest::create();
    let mut path = Path::<PathParameters>::extract(&test_request.request).await.unwrap();
    path.id = code.id;

    let start_date = NaiveDateTime::from(Utc::now().naive_utc() + Duration::days(1));
    let end_date = NaiveDateTime::from(Utc::now().naive_utc() - Duration::days(2));
    let json = Json(UpdateCodeRequest {
        redemption_codes: Some(vec!["a".into()]),
        start_date: Some(Some(start_date)),
        end_date: Some(Some(end_date)),
        ..Default::default()
    });

    let response: HttpResponse = codes::update((database.connection.clone().into(), json, path, auth_user))
        .await
        .into();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert!(response.error().is_some());

    let validation_response = support::validation_response_from_response(&response).unwrap();
    let start_date = validation_response.fields.get("start_date").unwrap();
    assert_eq!(start_date[0].code, "start_date_must_be_before_end_date");
    assert_eq!(
        &start_date[0].message.clone().unwrap().into_owned(),
        "Start date must be before end date"
    );
}

#[actix_rt::test]
async fn update_fails_adding_ticket_type_id_from_other_event() {
    let database = TestDatabase::new();
    let connection = database.connection.get();
    let user = database.create_user().finish();
    let event = database.create_event().finish();
    let event2 = database.create_event().with_ticket_pricing().finish();
    let ticket_type_id = event2.ticket_types(true, None, connection).unwrap()[0].id;
    let code = database.create_code().with_event(&event).finish();
    let organization = event.organization(connection).unwrap();
    let auth_user = support::create_auth_user_from_user(&user, Roles::OrgOwner, Some(&organization), &database);

    let test_request = TestRequest::create();
    let mut path = Path::<PathParameters>::extract(&test_request.request).await.unwrap();
    path.id = code.id;

    let json = Json(UpdateCodeRequest {
        ticket_type_ids: Some(vec![ticket_type_id]),
        ..Default::default()
    });

    let response: HttpResponse = codes::update((database.connection.clone().into(), json, path, auth_user))
        .await
        .into();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert!(response.error().is_some());

    let validation_response = support::validation_response_from_response(&response).unwrap();
    let ticket_type_id = validation_response.fields.get("ticket_type_id").unwrap();
    assert_eq!(ticket_type_id[0].code, "invalid");
    assert_eq!(
        &ticket_type_id[0].message.clone().unwrap().into_owned(),
        "Ticket type not valid for code as it does not belong to same event"
    );
}

#[actix_rt::test]
pub async fn update_adding_keeping_and_removing_ticket_types() {
    let database = TestDatabase::new();
    let connection = database.connection.get();
    let user = database.create_user().finish();
    let event = database
        .create_event()
        .with_ticket_pricing()
        .with_ticket_type_count(3)
        .finish();
    let ticket_types = event.ticket_types(true, None, connection).unwrap();
    let ticket_type = &ticket_types[0];
    let ticket_type2 = &ticket_types[1];
    let ticket_type3 = &ticket_types[2];
    let code = database
        .create_code()
        .with_event(&event)
        .for_ticket_type(&ticket_type)
        .for_ticket_type(&ticket_type2)
        .finish();
    let mut display_code = code.for_display(connection).unwrap();
    assert_eq!(
        display_code.display_code.ticket_type_ids.sort(),
        vec![ticket_type.id, ticket_type2.id].sort()
    );

    let organization = event.organization(connection).unwrap();
    let auth_user = support::create_auth_user_from_user(&user, Roles::OrgOwner, Some(&organization), &database);

    let test_request = TestRequest::create();
    let mut path = Path::<PathParameters>::extract(&test_request.request).await.unwrap();
    path.id = code.id;

    // Keep ticket_type, remove ticket_type2, add ticket_type3
    let json = Json(UpdateCodeRequest {
        ticket_type_ids: Some(vec![ticket_type.id, ticket_type3.id]),
        ..Default::default()
    });

    let response: HttpResponse = codes::update((database.connection.clone().into(), json, path, auth_user))
        .await
        .into();
    let body = support::unwrap_body_to_string(&response).unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let mut updated_code: DisplayCode = serde_json::from_str(&body).unwrap();
    assert_eq!(
        updated_code.ticket_type_ids.sort(),
        vec![ticket_type.id, ticket_type3.id].sort()
    );
}
