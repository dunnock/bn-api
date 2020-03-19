use crate::functional::base;
use crate::support;
use crate::support::database::TestDatabase;
use crate::support::test_request::TestRequest;
use actix_web::{http::StatusCode, web::Path, FromRequest, HttpResponse};
use api::controllers::notes::{self, *};
use api::extractors::*;
use api::models::*;
use db::prelude::*;

#[cfg(test)]
mod index_tests {
    use super::*;
    #[actix_rt::test]
    async fn index_org_member() {
        base::notes::index(Roles::OrgMember, false, true).await;
    }
    #[actix_rt::test]
    async fn index_admin() {
        base::notes::index(Roles::Admin, false, true).await;
    }
    #[actix_rt::test]
    async fn index_user() {
        base::notes::index(Roles::User, false, false).await;
    }
    #[actix_rt::test]
    async fn index_org_owner() {
        base::notes::index(Roles::OrgOwner, false, true).await;
    }
    #[actix_rt::test]
    async fn index_door_person() {
        base::notes::index(Roles::DoorPerson, false, true).await;
    }
    #[actix_rt::test]
    async fn index_promoter() {
        base::notes::index(Roles::Promoter, false, true).await;
    }
    #[actix_rt::test]
    async fn index_promoter_read_only() {
        base::notes::index(Roles::PromoterReadOnly, false, true).await;
    }
    #[actix_rt::test]
    async fn index_org_admin() {
        base::notes::index(Roles::OrgAdmin, false, true).await;
    }
    #[actix_rt::test]
    async fn index_box_office() {
        base::notes::index(Roles::OrgBoxOffice, false, true).await;
    }
    #[actix_rt::test]
    async fn index_filter_deleted_disabled_org_member() {
        base::notes::index(Roles::OrgMember, true, false).await;
    }
    #[actix_rt::test]
    async fn index_filter_deleted_disabled_admin() {
        base::notes::index(Roles::Admin, true, true).await;
    }
    #[actix_rt::test]
    async fn index_filter_deleted_disabled_user() {
        base::notes::index(Roles::User, true, false).await;
    }
    #[actix_rt::test]
    async fn index_filter_deleted_disabled_org_owner() {
        base::notes::index(Roles::OrgOwner, true, true).await;
    }
    #[actix_rt::test]
    async fn index_filter_deleted_disabled_door_person() {
        base::notes::index(Roles::DoorPerson, true, false).await;
    }
    #[actix_rt::test]
    async fn index_filter_deleted_disabled_promoter() {
        base::notes::index(Roles::Promoter, true, false).await;
    }
    #[actix_rt::test]
    async fn index_filter_deleted_disabled_promoter_read_only() {
        base::notes::index(Roles::PromoterReadOnly, true, false).await;
    }
    #[actix_rt::test]
    async fn index_filter_deleted_disabled_org_admin() {
        base::notes::index(Roles::OrgAdmin, true, true).await;
    }
    #[actix_rt::test]
    async fn index_filter_deleted_disabled_box_office() {
        base::notes::index(Roles::OrgBoxOffice, true, false).await;
    }
}

#[cfg(test)]
mod create_tests {
    use super::*;
    #[actix_rt::test]
    async fn create_org_member() {
        base::notes::create(Roles::OrgMember, true).await;
    }
    #[actix_rt::test]
    async fn create_admin() {
        base::notes::create(Roles::Admin, true).await;
    }
    #[actix_rt::test]
    async fn create_user() {
        base::notes::create(Roles::User, false).await;
    }
    #[actix_rt::test]
    async fn create_org_owner() {
        base::notes::create(Roles::OrgOwner, true).await;
    }
    #[actix_rt::test]
    async fn create_door_person() {
        base::notes::create(Roles::DoorPerson, true).await;
    }
    #[actix_rt::test]
    async fn create_promoter() {
        base::notes::create(Roles::Promoter, true).await;
    }
    #[actix_rt::test]
    async fn create_promoter_read_only() {
        base::notes::create(Roles::PromoterReadOnly, false).await;
    }
    #[actix_rt::test]
    async fn create_org_admin() {
        base::notes::create(Roles::OrgAdmin, true).await;
    }
    #[actix_rt::test]
    async fn create_box_office() {
        base::notes::create(Roles::OrgBoxOffice, true).await;
    }
}

#[cfg(test)]
mod destroy_tests {
    use super::*;
    #[actix_rt::test]
    async fn destroy_org_member() {
        base::notes::destroy(Roles::OrgMember, false).await;
    }
    #[actix_rt::test]
    async fn destroy_admin() {
        base::notes::destroy(Roles::Admin, true).await;
    }
    #[actix_rt::test]
    async fn destroy_user() {
        base::notes::destroy(Roles::User, false).await;
    }
    #[actix_rt::test]
    async fn destroy_org_owner() {
        base::notes::destroy(Roles::OrgOwner, true).await;
    }
    #[actix_rt::test]
    async fn destroy_door_person() {
        base::notes::destroy(Roles::DoorPerson, false).await;
    }
    #[actix_rt::test]
    async fn destroy_promoter() {
        base::notes::destroy(Roles::Promoter, false).await;
    }
    #[actix_rt::test]
    async fn destroy_promoter_read_only() {
        base::notes::destroy(Roles::PromoterReadOnly, false).await;
    }
    #[actix_rt::test]
    async fn destroy_org_admin() {
        base::notes::destroy(Roles::OrgAdmin, true).await;
    }
    #[actix_rt::test]
    async fn destroy_box_office() {
        base::notes::destroy(Roles::OrgBoxOffice, false).await;
    }
}

#[actix_rt::test]
async fn create_with_validation_errors() {
    let database = TestDatabase::new();
    let user = database.create_user().finish();
    let organization = database.create_organization().with_event_fee().with_fees().finish();
    let event = database
        .create_event()
        .with_organization(&organization)
        .with_tickets()
        .with_ticket_pricing()
        .finish();

    let order = database.create_order().for_event(&event).is_paid().finish();
    let auth_user = support::create_auth_user_from_user(&user, Roles::OrgAdmin, Some(&organization), &database);

    let json = Json(NewNoteRequest { note: "".to_string() });

    let test_request = TestRequest::create_with_uri_custom_params("/", vec!["main_table", "id"]);
    let mut path = Path::<MainTablePathParameters>::extract(&test_request.request)
        .await
        .unwrap();
    path.id = order.id;
    path.main_table = Tables::Orders.to_string();

    let response: HttpResponse = notes::create((database.connection.into(), path, json, auth_user))
        .await
        .into();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert!(response.error().is_some());
    let validation_response = support::validation_response_from_response(&response).unwrap();
    let note = validation_response.fields.get("note").unwrap();
    assert_eq!(note[0].code, "length");
    assert_eq!(&note[0].message.clone().unwrap().into_owned(), "Note cannot be blank");
}
