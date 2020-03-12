use crate::functional::base;
use crate::support;
use crate::support::database::TestDatabase;
use crate::support::test_request::TestRequest;
use actix_web::{http::StatusCode, HttpResponse, web::{Path, Query}};
use bigneon_api::controllers::regions;
use bigneon_api::models::PathParameters;
use bigneon_db::models::*;
use serde_json;
use uuid::Uuid;

#[test]
fn index() {
    let database = TestDatabase::new();
    let region = database.create_region().with_name("Region1".into()).finish();
    let region2 = database.create_region().with_name("Region2".into()).finish();

    let expected_regions = vec![region, region2];
    let test_request = TestRequest::create_with_uri(&format!("/limits?"));
    let query_parameters = Query::<PagingParameters>::extract(&test_request.request).unwrap();
    let response = regions::index((database.connection.into(), query_parameters)).unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.payload().data[0].id, Uuid::nil());
    assert_eq!(response.payload().data[1], expected_regions[0]);
    assert_eq!(response.payload().data[2], expected_regions[1]);
}

#[test]
fn show() {
    let database = TestDatabase::new();
    let region = database.create_region().finish();
    let region_expected_json = serde_json::to_string(&region).unwrap();

    let test_request = TestRequest::create();
    let mut path = Path::<PathParameters>::extract(&test_request.request).unwrap();
    path.id = region.id;

    let response: HttpResponse = regions::show((database.connection.into(), path)).into();

    assert_eq!(response.status(), StatusCode::OK);
    let body = support::unwrap_body_to_string(&response).unwrap();
    assert_eq!(body, region_expected_json);
}

#[cfg(test)]
mod create_tests {
    use super::*;
    #[test]
    fn create_org_member() {
        base::regions::create(Roles::OrgMember, false);
    }
    #[test]
    fn create_admin() {
        base::regions::create(Roles::Admin, true);
    }
    #[test]
    fn create_user() {
        base::regions::create(Roles::User, false);
    }
    #[test]
    fn create_org_owner() {
        base::regions::create(Roles::OrgOwner, false);
    }
    #[test]
    fn create_door_person() {
        base::regions::create(Roles::DoorPerson, false);
    }
    #[test]
    fn create_promoter() {
        base::regions::create(Roles::Promoter, false);
    }
    #[test]
    fn create_promoter_read_only() {
        base::regions::create(Roles::PromoterReadOnly, false);
    }
    #[test]
    fn create_org_admin() {
        base::regions::create(Roles::OrgAdmin, false);
    }
    #[test]
    fn create_box_office() {
        base::regions::create(Roles::OrgBoxOffice, false);
    }
}

#[cfg(test)]
mod update_tests {
    use super::*;
    #[test]
    fn update_org_member() {
        base::regions::update(Roles::OrgMember, false);
    }
    #[test]
    fn update_admin() {
        base::regions::update(Roles::Admin, true);
    }
    #[test]
    fn update_user() {
        base::regions::update(Roles::User, false);
    }
    #[test]
    fn update_org_owner() {
        base::regions::update(Roles::OrgOwner, false);
    }
    #[test]
    fn update_door_person() {
        base::regions::update(Roles::DoorPerson, false);
    }
    #[test]
    fn update_promoter() {
        base::regions::update(Roles::Promoter, false);
    }
    #[test]
    fn update_promoter_read_only() {
        base::regions::update(Roles::PromoterReadOnly, false);
    }
    #[test]
    fn update_org_admin() {
        base::regions::update(Roles::OrgAdmin, false);
    }
    #[test]
    fn update_box_office() {
        base::regions::update(Roles::OrgBoxOffice, false);
    }
}
