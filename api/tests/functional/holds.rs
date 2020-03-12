use crate::functional::base;
use crate::support;
use crate::support::database::TestDatabase;
use crate::support::test_request::TestRequest;
use actix_web::{http::StatusCode, web::Path, FromRequest, HttpResponse};
use bigneon_api::controllers::holds::{self, *};
use bigneon_api::extractors::*;
use bigneon_api::models::PathParameters;
use bigneon_db::models::*;
use uuid::Uuid;

#[cfg(test)]
mod create_tests {
    use super::*;
    #[actix_rt::test]
    async fn create_org_member() {
        base::holds::create(Roles::OrgMember, true).await;
    }
    #[actix_rt::test]
    async fn create_admin() {
        base::holds::create(Roles::Admin, true).await;
    }
    #[actix_rt::test]
    async fn create_user() {
        base::holds::create(Roles::User, false).await;
    }
    #[actix_rt::test]
    async fn create_org_owner() {
        base::holds::create(Roles::OrgOwner, true).await;
    }
    #[actix_rt::test]
    async fn create_door_person() {
        base::holds::create(Roles::DoorPerson, false).await;
    }
    #[actix_rt::test]
    async fn create_promoter() {
        base::holds::create(Roles::Promoter, true).await;
    }
    #[actix_rt::test]
    async fn create_promoter_read_only() {
        base::holds::create(Roles::PromoterReadOnly, false).await;
    }
    #[actix_rt::test]
    async fn create_org_admin() {
        base::holds::create(Roles::OrgAdmin, true).await;
    }
    #[actix_rt::test]
    async fn create_box_office() {
        base::holds::create(Roles::OrgBoxOffice, false).await;
    }
}

#[cfg(test)]
mod children_tests {
    use super::*;
    #[actix_rt::test]
    async fn children_org_member() {
        base::holds::children(Roles::OrgMember, true).await;
    }
    #[actix_rt::test]
    async fn children_admin() {
        base::holds::children(Roles::Admin, true).await;
    }
    #[actix_rt::test]
    async fn children_user() {
        base::holds::children(Roles::User, false).await;
    }
    #[actix_rt::test]
    async fn children_org_owner() {
        base::holds::children(Roles::OrgOwner, true).await;
    }
    #[actix_rt::test]
    async fn children_door_person() {
        base::holds::children(Roles::DoorPerson, true).await;
    }
    #[actix_rt::test]
    async fn children_promoter() {
        base::holds::children(Roles::Promoter, true).await;
    }
    #[actix_rt::test]
    async fn children_promoter_read_only() {
        base::holds::children(Roles::PromoterReadOnly, true).await;
    }
    #[actix_rt::test]
    async fn children_org_admin() {
        base::holds::children(Roles::OrgAdmin, true).await;
    }
    #[actix_rt::test]
    async fn children_box_office() {
        base::holds::children(Roles::OrgBoxOffice, true).await;
    }
}

#[cfg(test)]
mod split_tests {
    use super::*;
    #[actix_rt::test]
    async fn split_org_member() {
        base::holds::split(Roles::OrgMember, true).await;
    }
    #[actix_rt::test]
    async fn split_admin() {
        base::holds::split(Roles::Admin, true).await;
    }
    #[actix_rt::test]
    async fn split_user() {
        base::holds::split(Roles::User, false).await;
    }
    #[actix_rt::test]
    async fn split_org_owner() {
        base::holds::split(Roles::OrgOwner, true).await;
    }
    #[actix_rt::test]
    async fn split_door_person() {
        base::holds::split(Roles::DoorPerson, false).await;
    }
    #[actix_rt::test]
    async fn split_promoter() {
        base::holds::split(Roles::Promoter, true).await;
    }
    #[actix_rt::test]
    async fn split_promoter_read_only() {
        base::holds::split(Roles::PromoterReadOnly, false).await;
    }
    #[actix_rt::test]
    async fn split_org_admin() {
        base::holds::split(Roles::OrgAdmin, true).await;
    }
    #[actix_rt::test]
    async fn split_box_office() {
        base::holds::split(Roles::OrgBoxOffice, false).await;
    }
}

#[cfg(test)]
mod update_tests {
    use super::*;
    #[actix_rt::test]
    async fn update_org_member() {
        base::holds::update(Roles::OrgMember, true).await;
    }
    #[actix_rt::test]
    async fn update_admin() {
        base::holds::update(Roles::Admin, true).await;
    }
    #[actix_rt::test]
    async fn update_user() {
        base::holds::update(Roles::User, false).await;
    }
    #[actix_rt::test]
    async fn update_org_owner() {
        base::holds::update(Roles::OrgOwner, true).await;
    }
    #[actix_rt::test]
    async fn update_door_person() {
        base::holds::update(Roles::DoorPerson, false).await;
    }
    #[actix_rt::test]
    async fn update_promoter() {
        base::holds::update(Roles::Promoter, true).await;
    }
    #[actix_rt::test]
    async fn update_promoter_read_only() {
        base::holds::update(Roles::PromoterReadOnly, false).await;
    }
    #[actix_rt::test]
    async fn update_org_admin() {
        base::holds::update(Roles::OrgAdmin, true).await;
    }
    #[actix_rt::test]
    async fn update_box_office() {
        base::holds::update(Roles::OrgBoxOffice, false).await;
    }
}

#[actix_rt::test]
async fn create_with_validation_errors() {
    let database = TestDatabase::new();
    let organization = database.create_organization().finish();
    let user = database.create_user().finish();
    let auth_user = support::create_auth_user_from_user(&user, Roles::OrgOwner, Some(&organization), &database);
    let event = database
        .create_event()
        .with_tickets()
        .with_organization(&organization)
        .finish();

    let name = "Hold Example".to_string();
    let redemption_code = "IHAVEACODE".to_string();
    let hold_type = HoldTypes::Discount;

    let json = Json(CreateHoldRequest {
        name: name.clone(),
        redemption_code: Some(redemption_code),
        discount_in_cents: None,
        hold_type,
        end_at: None,
        max_per_user: None,
        quantity: 2,
        ticket_type_id: event.ticket_types(true, None, database.connection.get()).unwrap()[0].id,
    });

    let test_request = TestRequest::create();
    let mut path = Path::<PathParameters>::extract(&test_request.request).await.unwrap();
    path.id = event.id;

    let response: HttpResponse = holds::create((database.connection.into(), json, path, auth_user))
        .await
        .into();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert!(response.error().is_some());

    let validation_response = support::validation_response_from_response(&response).unwrap();
    let discount_in_cents = validation_response.fields.get("discount_in_cents").unwrap();
    assert_eq!(discount_in_cents[0].code, "required");
    assert_eq!(
        &discount_in_cents[0].message.clone().unwrap().into_owned(),
        "Discount required for hold type Discount"
    );
}

#[actix_rt::test]
async fn update_with_validation_errors() {
    let database = TestDatabase::new();
    let connection = database.connection.get();
    let user = database.create_user().finish();
    let hold = database.create_hold().with_hold_type(HoldTypes::Comp).finish();
    let event = Event::find(hold.event_id, connection).unwrap();
    let organization = event.organization(connection).unwrap();
    let auth_user = support::create_auth_user_from_user(&user, Roles::OrgOwner, Some(&organization), &database);
    let name = "New Name";

    let test_request = TestRequest::create();
    let mut path = Path::<PathParameters>::extract(&test_request.request).await.unwrap();
    path.id = hold.id;

    let json = Json(UpdateHoldRequest {
        name: Some(name.into()),
        hold_type: Some(HoldTypes::Discount),
        ..Default::default()
    });

    let response: HttpResponse = holds::update((database.connection.clone(), json, path, auth_user))
        .await
        .into();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert!(response.error().is_some());

    let validation_response = support::validation_response_from_response(&response).unwrap();
    let discount_in_cents = validation_response.fields.get("discount_in_cents").unwrap();
    assert_eq!(discount_in_cents[0].code, "required");
    assert_eq!(
        &discount_in_cents[0].message.clone().unwrap().into_owned(),
        "Discount required for hold type Discount"
    );
}

#[actix_rt::test]
pub async fn read_hold() {
    let database = TestDatabase::new();
    let organization = database.create_organization().finish();
    let user = database.create_user().finish();
    let auth_user = support::create_auth_user_from_user(&user, Roles::OrgOwner, Some(&organization), &database);
    let event = database
        .create_event()
        .with_tickets()
        .with_organization(&organization)
        .finish();

    let name = "Hold Example".to_string();
    let redemption_code = "IHAVEACODE".to_string();
    let hold_type = HoldTypes::Discount;

    let json = Json(CreateHoldRequest {
        name: name.clone(),
        redemption_code: Some(redemption_code),
        discount_in_cents: Some(100),
        hold_type,
        end_at: None,
        max_per_user: None,
        quantity: 2,
        ticket_type_id: event.ticket_types(true, None, database.connection.get()).unwrap()[0].id,
    });

    let test_request = TestRequest::create();
    let mut path = Path::<PathParameters>::extract(&test_request.request).await.unwrap();
    path.id = event.id;

    let response: HttpResponse = holds::create((database.connection.clone().into(), json, path, auth_user.clone()))
        .await
        .into();
    let body = support::unwrap_body_to_string(&response).unwrap();
    let created_hold: DisplayHold = serde_json::from_str(body).unwrap();

    let mut hold_path = Path::<PathParameters>::extract(&test_request.request).await.unwrap();

    hold_path.id = created_hold.id;
    let show_response = holds::show((database.connection.into(), hold_path, auth_user))
        .await
        .into();
    let show_body = support::unwrap_body_to_string(&show_response).unwrap();

    #[derive(Deserialize)]
    struct R {
        id: Uuid,
    }
    let fetched_hold: R = serde_json::from_str(show_body).unwrap();

    assert_eq!(created_hold.id, fetched_hold.id);
}
