use crate::functional::base;
use crate::support;
use crate::support::database::TestDatabase;
use crate::support::test_request::TestRequest;
use actix_web::{
    http::StatusCode,
    web::{Path, Query},
    FromRequest, HttpResponse,
};
use api::controllers::regions;
use api::models::PathParameters;
use db::models::*;
use serde_json;
use uuid::Uuid;

#[actix_rt::test]
async fn index() {
    let database = TestDatabase::new();
    let region = database.create_region().with_name("Region1".into()).finish();
    let region2 = database.create_region().with_name("Region2".into()).finish();

    let expected_regions = vec![region, region2];
    let test_request = TestRequest::create_with_uri(&format!("/limits?"));
    let query_parameters = Query::<PagingParameters>::extract(&test_request.request).await.unwrap();
    let response = regions::index((database.connection.into(), query_parameters))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.payload().data[0].id, Uuid::nil());
    assert_eq!(response.payload().data[1], expected_regions[0]);
    assert_eq!(response.payload().data[2], expected_regions[1]);
}

#[actix_rt::test]
async fn show() {
    let database = TestDatabase::new();
    let region = database.create_region().finish();
    let region_expected_json = serde_json::to_string(&region).unwrap();

    let test_request = TestRequest::create();
    let mut path = Path::<PathParameters>::extract(&test_request.request).await.unwrap();
    path.id = region.id;

    let response: HttpResponse = regions::show((database.connection.into(), path)).await.into();

    assert_eq!(response.status(), StatusCode::OK);
    let body = support::unwrap_body_to_string(&response).unwrap();
    assert_eq!(body, region_expected_json);
}

#[cfg(test)]
mod create_tests {
    use super::*;
    #[actix_rt::test]
    async fn create_org_member() {
        base::regions::create(Roles::OrgMember, false).await;
    }
    #[actix_rt::test]
    async fn create_admin() {
        base::regions::create(Roles::Admin, true).await;
    }
    #[actix_rt::test]
    async fn create_user() {
        base::regions::create(Roles::User, false).await;
    }
    #[actix_rt::test]
    async fn create_org_owner() {
        base::regions::create(Roles::OrgOwner, false).await;
    }
    #[actix_rt::test]
    async fn create_door_person() {
        base::regions::create(Roles::DoorPerson, false).await;
    }
    #[actix_rt::test]
    async fn create_promoter() {
        base::regions::create(Roles::Promoter, false).await;
    }
    #[actix_rt::test]
    async fn create_promoter_read_only() {
        base::regions::create(Roles::PromoterReadOnly, false).await;
    }
    #[actix_rt::test]
    async fn create_org_admin() {
        base::regions::create(Roles::OrgAdmin, false).await;
    }
    #[actix_rt::test]
    async fn create_box_office() {
        base::regions::create(Roles::OrgBoxOffice, false).await;
    }
}

#[cfg(test)]
mod update_tests {
    use super::*;
    #[actix_rt::test]
    async fn update_org_member() {
        base::regions::update(Roles::OrgMember, false).await;
    }
    #[actix_rt::test]
    async fn update_admin() {
        base::regions::update(Roles::Admin, true).await;
    }
    #[actix_rt::test]
    async fn update_user() {
        base::regions::update(Roles::User, false).await;
    }
    #[actix_rt::test]
    async fn update_org_owner() {
        base::regions::update(Roles::OrgOwner, false).await;
    }
    #[actix_rt::test]
    async fn update_door_person() {
        base::regions::update(Roles::DoorPerson, false).await;
    }
    #[actix_rt::test]
    async fn update_promoter() {
        base::regions::update(Roles::Promoter, false).await;
    }
    #[actix_rt::test]
    async fn update_promoter_read_only() {
        base::regions::update(Roles::PromoterReadOnly, false).await;
    }
    #[actix_rt::test]
    async fn update_org_admin() {
        base::regions::update(Roles::OrgAdmin, false).await;
    }
    #[actix_rt::test]
    async fn update_box_office() {
        base::regions::update(Roles::OrgBoxOffice, false).await;
    }
}
