use actix_web::{http::StatusCode, FromRequest, HttpResponse, Path};
use bigneon_api::controllers::holds::{self, *};
use bigneon_api::extractors::*;
use bigneon_api::models::PathParameters;
use bigneon_db::models::*;
use crate::functional::base;
use crate::support;
use crate::support::database::TestDatabase;
use crate::support::test_request::TestRequest;
use uuid::Uuid;

#[cfg(test)]
mod create_tests {
    use super::*;
    #[test]
    fn create_org_member() {
        base::holds::create(Roles::OrgMember, true);
    }
    #[test]
    fn create_admin() {
        base::holds::create(Roles::Admin, true);
    }
    #[test]
    fn create_user() {
        base::holds::create(Roles::User, false);
    }
    #[test]
    fn create_org_owner() {
        base::holds::create(Roles::OrgOwner, true);
    }
    #[test]
    fn create_door_person() {
        base::holds::create(Roles::DoorPerson, false);
    }
    #[test]
    fn create_promoter() {
        base::holds::create(Roles::Promoter, true);
    }
    #[test]
    fn create_promoter_read_only() {
        base::holds::create(Roles::PromoterReadOnly, false);
    }
    #[test]
    fn create_org_admin() {
        base::holds::create(Roles::OrgAdmin, true);
    }
    #[test]
    fn create_box_office() {
        base::holds::create(Roles::OrgBoxOffice, false);
    }
}

#[cfg(test)]
mod children_tests {
    use super::*;
    #[test]
    fn children_org_member() {
        base::holds::children(Roles::OrgMember, true);
    }
    #[test]
    fn children_admin() {
        base::holds::children(Roles::Admin, true);
    }
    #[test]
    fn children_user() {
        base::holds::children(Roles::User, false);
    }
    #[test]
    fn children_org_owner() {
        base::holds::children(Roles::OrgOwner, true);
    }
    #[test]
    fn children_door_person() {
        base::holds::children(Roles::DoorPerson, true);
    }
    #[test]
    fn children_promoter() {
        base::holds::children(Roles::Promoter, true);
    }
    #[test]
    fn children_promoter_read_only() {
        base::holds::children(Roles::PromoterReadOnly, true);
    }
    #[test]
    fn children_org_admin() {
        base::holds::children(Roles::OrgAdmin, true);
    }
    #[test]
    fn children_box_office() {
        base::holds::children(Roles::OrgBoxOffice, true);
    }
}

#[cfg(test)]
mod split_tests {
    use super::*;
    #[test]
    fn split_org_member() {
        base::holds::split(Roles::OrgMember, true);
    }
    #[test]
    fn split_admin() {
        base::holds::split(Roles::Admin, true);
    }
    #[test]
    fn split_user() {
        base::holds::split(Roles::User, false);
    }
    #[test]
    fn split_org_owner() {
        base::holds::split(Roles::OrgOwner, true);
    }
    #[test]
    fn split_door_person() {
        base::holds::split(Roles::DoorPerson, false);
    }
    #[test]
    fn split_promoter() {
        base::holds::split(Roles::Promoter, true);
    }
    #[test]
    fn split_promoter_read_only() {
        base::holds::split(Roles::PromoterReadOnly, false);
    }
    #[test]
    fn split_org_admin() {
        base::holds::split(Roles::OrgAdmin, true);
    }
    #[test]
    fn split_box_office() {
        base::holds::split(Roles::OrgBoxOffice, false);
    }
}

#[cfg(test)]
mod update_tests {
    use super::*;
    #[test]
    fn update_org_member() {
        base::holds::update(Roles::OrgMember, true);
    }
    #[test]
    fn update_admin() {
        base::holds::update(Roles::Admin, true);
    }
    #[test]
    fn update_user() {
        base::holds::update(Roles::User, false);
    }
    #[test]
    fn update_org_owner() {
        base::holds::update(Roles::OrgOwner, true);
    }
    #[test]
    fn update_door_person() {
        base::holds::update(Roles::DoorPerson, false);
    }
    #[test]
    fn update_promoter() {
        base::holds::update(Roles::Promoter, true);
    }
    #[test]
    fn update_promoter_read_only() {
        base::holds::update(Roles::PromoterReadOnly, false);
    }
    #[test]
    fn update_org_admin() {
        base::holds::update(Roles::OrgAdmin, true);
    }
    #[test]
    fn update_box_office() {
        base::holds::update(Roles::OrgBoxOffice, false);
    }
}

#[test]
fn create_with_validation_errors() {
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
    let mut path = Path::<PathParameters>::extract(&test_request.request).unwrap();
    path.id = event.id;

    let response: HttpResponse = holds::create((database.connection.into(), json, path, auth_user)).into();
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

#[test]
fn update_with_validation_errors() {
    let database = TestDatabase::new();
    let connection = database.connection.get();
    let user = database.create_user().finish();
    let hold = database.create_hold().with_hold_type(HoldTypes::Comp).finish();
    let event = Event::find(hold.event_id, connection).unwrap();
    let organization = event.organization(connection).unwrap();
    let auth_user = support::create_auth_user_from_user(&user, Roles::OrgOwner, Some(&organization), &database);
    let name = "New Name";

    let test_request = TestRequest::create();
    let mut path = Path::<PathParameters>::extract(&test_request.request).unwrap();
    path.id = hold.id;

    let json = Json(UpdateHoldRequest {
        name: Some(name.into()),
        hold_type: Some(HoldTypes::Discount),
        ..Default::default()
    });

    let response: HttpResponse = holds::update((database.connection.clone(), json, path, auth_user)).into();
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

#[test]
pub fn read_hold() {
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
    let mut path = Path::<PathParameters>::extract(&test_request.request).unwrap();
    path.id = event.id;

    let response: HttpResponse =
        holds::create((database.connection.clone().into(), json, path, auth_user.clone())).into();
    let body = support::unwrap_body_to_string(&response).unwrap();
    let created_hold: DisplayHold = serde_json::from_str(body).unwrap();

    let mut hold_path = Path::<PathParameters>::extract(&test_request.request).unwrap();

    hold_path.id = created_hold.id;
    let show_response = holds::show((database.connection.into(), hold_path, auth_user)).into();
    let show_body = support::unwrap_body_to_string(&show_response).unwrap();

    #[derive(Deserialize)]
    struct R {
        id: Uuid,
    }
    let fetched_hold: R = serde_json::from_str(show_body).unwrap();

    assert_eq!(created_hold.id, fetched_hold.id);
}
