use crate::functional::base;
use crate::support;
use crate::support::database::TestDatabase;
use crate::support::test_request::TestRequest;
use actix_web::{
    http::StatusCode,
    web::{Path, Query},
    FromRequest, HttpResponse,
};
use api::controllers::venues;
use api::extractors::*;
use api::models::PathParameters;
use db::models::*;
use serde_json;
use std::collections::HashMap;

#[actix_rt::test]
async fn index_with_org_linked_and_private_venues() {
    let database = TestDatabase::new();
    let venue = database.create_venue().with_name("Venue1".to_string()).finish();
    let venue2 = database.create_venue().with_name("Venue2".to_string()).finish();

    let organization = database.create_organization().finish();
    let auth_user = support::create_auth_user(Roles::User, Some(&organization), &database);
    let venue3 = database.create_venue().with_name("Venue3".to_string()).finish();
    venue3
        .add_to_organization(organization.id, database.connection.get())
        .unwrap();

    let venue4 = database
        .create_venue()
        .make_private()
        .with_name("Venue4".to_string())
        .finish();
    venue4
        .add_to_organization(organization.id, database.connection.get())
        .unwrap();
    let test_request = TestRequest::create_with_uri(&format!("/limits?"));
    let query_parameters = Query::<PagingParameters>::extract(&test_request.request).await.unwrap();
    //first try with no user
    let response: HttpResponse =
        venues::index((database.connection.clone().into(), query_parameters, OptionalUser(None)))
            .await
            .into();

    let mut expected_venues = vec![venue, venue2, venue3];
    let wrapped_expected_venues = Payload {
        data: expected_venues.clone(),
        paging: Paging {
            page: 0,
            limit: 100,
            sort: "".to_string(),
            dir: SortingDir::Asc,
            total: 3,
            tags: HashMap::new(),
        },
    };
    let expected_json = serde_json::to_string(&wrapped_expected_venues).unwrap();

    let body = support::unwrap_body_to_string(&response).unwrap();
    assert_eq!(body, expected_json);
    let query_parameters = Query::<PagingParameters>::extract(&test_request.request).await.unwrap();
    //now try with user that does not belong to org
    let response: HttpResponse = venues::index((
        database.connection.clone().into(),
        query_parameters,
        OptionalUser(Some(auth_user.clone())),
    ))
    .await
    .into();

    let body = support::unwrap_body_to_string(&response).unwrap();
    assert_eq!(body, expected_json);

    //now with user that DOES belong to org
    let _ = organization.add_user(
        auth_user.id(),
        vec![Roles::OrgMember],
        Vec::new(),
        database.connection.get(),
    );
    expected_venues.push(venue4);

    let query_parameters = Query::<PagingParameters>::extract(&test_request.request).await.unwrap();
    let response: HttpResponse = venues::index((
        database.connection.into(),
        query_parameters,
        OptionalUser(Some(auth_user)),
    ))
    .await
    .into();
    let wrapped_expected_venues = Payload {
        data: expected_venues,
        paging: Paging {
            page: 0,
            limit: 100,
            sort: "".to_string(),
            dir: SortingDir::Asc,
            total: 4,
            tags: HashMap::new(),
        },
    };
    let expected_json = serde_json::to_string(&wrapped_expected_venues).unwrap();
    let body = support::unwrap_body_to_string(&response).unwrap();
    assert_eq!(body, expected_json);
}

#[actix_rt::test]
pub async fn show() {
    let database = TestDatabase::new();
    let venue = database.create_venue().finish();
    let venue_expected_json = serde_json::to_string(&venue).unwrap();

    let test_request = TestRequest::create();
    let mut path = Path::<PathParameters>::extract(&test_request.request).await.unwrap();
    path.id = venue.id;

    let response: HttpResponse = venues::show((database.connection.into(), path)).await.into();
    assert_eq!(response.status(), StatusCode::OK);
    let body = support::unwrap_body_to_string(&response).unwrap();
    assert_eq!(body, venue_expected_json);
}

#[cfg(test)]
mod index_tests {
    use super::*;
    #[actix_rt::test]
    async fn index_org_member() {
        base::venues::index(Roles::OrgMember, true).await;
    }
    #[actix_rt::test]
    async fn index_admin() {
        base::venues::index(Roles::Admin, true).await;
    }
    #[actix_rt::test]
    async fn index_user() {
        base::venues::index(Roles::User, true).await;
    }
    #[actix_rt::test]
    async fn index_org_owner() {
        base::venues::index(Roles::OrgOwner, true).await;
    }
    #[actix_rt::test]
    async fn index_door_person() {
        base::venues::index(Roles::DoorPerson, true).await;
    }
    #[actix_rt::test]
    async fn index_promoter() {
        base::venues::index(Roles::Promoter, true).await;
    }
    #[actix_rt::test]
    async fn index_promoter_read_only() {
        base::venues::index(Roles::PromoterReadOnly, true).await;
    }
    #[actix_rt::test]
    async fn index_org_admin() {
        base::venues::index(Roles::OrgAdmin, true).await;
    }
    #[actix_rt::test]
    async fn index_box_office() {
        base::venues::index(Roles::OrgBoxOffice, true).await;
    }
}

#[cfg(test)]
mod create_tests {
    use super::*;
    #[actix_rt::test]
    async fn create_org_member() {
        base::venues::create(Roles::OrgMember, true).await;
    }
    #[actix_rt::test]
    async fn create_admin() {
        base::venues::create(Roles::Admin, true).await;
    }
    #[actix_rt::test]
    async fn create_user() {
        base::venues::create(Roles::User, false).await;
    }
    #[actix_rt::test]
    async fn create_org_owner() {
        base::venues::create(Roles::OrgOwner, true).await;
    }
    #[actix_rt::test]
    async fn create_door_person() {
        base::venues::create(Roles::DoorPerson, false).await;
    }
    #[actix_rt::test]
    async fn create_promoter() {
        base::venues::create(Roles::Promoter, false).await;
    }
    #[actix_rt::test]
    async fn create_promoter_read_only() {
        base::venues::create(Roles::PromoterReadOnly, false).await;
    }
    #[actix_rt::test]
    async fn create_org_admin() {
        base::venues::create(Roles::OrgAdmin, true).await;
    }
    #[actix_rt::test]
    async fn create_box_office() {
        base::venues::create(Roles::OrgBoxOffice, false).await;
    }
}

#[cfg(test)]
mod toggle_privacy_tests {
    use super::*;
    #[actix_rt::test]
    async fn toggle_privacy_org_member() {
        base::venues::toggle_privacy(Roles::OrgMember, false).await;
    }
    #[actix_rt::test]
    async fn toggle_privacy_admin() {
        base::venues::toggle_privacy(Roles::Admin, true).await;
    }
    #[actix_rt::test]
    async fn toggle_privacy_user() {
        base::venues::toggle_privacy(Roles::User, false).await;
    }
    #[actix_rt::test]
    async fn toggle_privacy_org_owner() {
        base::venues::toggle_privacy(Roles::OrgOwner, false).await;
    }
    #[actix_rt::test]
    async fn toggle_privacy_door_person() {
        base::venues::toggle_privacy(Roles::DoorPerson, false).await;
    }
    #[actix_rt::test]
    async fn toggle_privacy_promoter() {
        base::venues::toggle_privacy(Roles::Promoter, false).await;
    }
    #[actix_rt::test]
    async fn toggle_privacy_promoter_read_only() {
        base::venues::toggle_privacy(Roles::PromoterReadOnly, false).await;
    }
    #[actix_rt::test]
    async fn toggle_privacy_org_admin() {
        base::venues::toggle_privacy(Roles::OrgAdmin, false).await;
    }
    #[actix_rt::test]
    async fn toggle_privacy_box_office() {
        base::venues::toggle_privacy(Roles::OrgBoxOffice, false).await;
    }
}

#[cfg(test)]
mod update_tests {
    use super::*;
    #[actix_rt::test]
    async fn update_org_member() {
        base::venues::update(Roles::OrgMember, false).await;
    }
    #[actix_rt::test]
    async fn update_admin() {
        base::venues::update(Roles::Admin, true).await;
    }
    #[actix_rt::test]
    async fn update_user() {
        base::venues::update(Roles::User, false).await;
    }
    #[actix_rt::test]
    async fn update_org_owner() {
        base::venues::update(Roles::OrgOwner, false).await;
    }
    #[actix_rt::test]
    async fn update_door_person() {
        base::venues::update(Roles::DoorPerson, false).await;
    }
    #[actix_rt::test]
    async fn update_promoter() {
        base::venues::update(Roles::Promoter, false).await;
    }
    #[actix_rt::test]
    async fn update_promoter_read_only() {
        base::venues::update(Roles::PromoterReadOnly, false).await;
    }
    #[actix_rt::test]
    async fn update_org_admin() {
        base::venues::update(Roles::OrgAdmin, false).await;
    }
    #[actix_rt::test]
    async fn update_box_office() {
        base::venues::update(Roles::OrgBoxOffice, false).await;
    }
}

#[cfg(test)]
mod update_with_organization_tests {
    use super::*;
    #[actix_rt::test]
    async fn update_with_organization_org_member() {
        base::venues::update_with_organization(Roles::OrgMember, true, true).await;
    }
    #[actix_rt::test]
    async fn update_with_organization_admin() {
        base::venues::update_with_organization(Roles::Admin, true, true).await;
    }
    #[actix_rt::test]
    async fn update_with_organization_user() {
        base::venues::update_with_organization(Roles::User, false, true).await;
    }
    #[actix_rt::test]
    async fn update_with_organization_org_owner() {
        base::venues::update_with_organization(Roles::OrgOwner, true, true).await;
    }
    #[actix_rt::test]
    async fn update_with_organization_door_person() {
        base::venues::update_with_organization(Roles::DoorPerson, false, true).await;
    }
    #[actix_rt::test]
    async fn update_with_organization_promoter() {
        base::venues::update_with_organization(Roles::Promoter, false, true).await;
    }
    #[actix_rt::test]
    async fn update_with_organization_promoter_read_only() {
        base::venues::update_with_organization(Roles::PromoterReadOnly, false, true).await;
    }
    #[actix_rt::test]
    async fn update_with_organization_org_admin() {
        base::venues::update_with_organization(Roles::OrgAdmin, true, true).await;
    }
    #[actix_rt::test]
    async fn update_with_organization_box_office() {
        base::venues::update_with_organization(Roles::OrgBoxOffice, false, true).await;
    }
}

#[cfg(test)]
mod update_public_venue_with_organization_tests {
    use super::*;
    #[actix_rt::test]
    async fn update_public_venue_with_organization_org_member() {
        base::venues::update_with_organization(Roles::OrgMember, false, false).await;
    }
    #[actix_rt::test]
    async fn update_public_venue_with_organization_admin() {
        base::venues::update_with_organization(Roles::Admin, true, false).await;
    }
    #[actix_rt::test]
    async fn update_public_venue_with_organization_user() {
        base::venues::update_with_organization(Roles::User, false, false).await;
    }
    #[actix_rt::test]
    async fn update_public_venue_with_organization_org_owner() {
        base::venues::update_with_organization(Roles::OrgOwner, false, false).await;
    }
    #[actix_rt::test]
    async fn update_public_with_organization_door_person() {
        base::venues::update_with_organization(Roles::DoorPerson, false, false).await;
    }
    #[actix_rt::test]
    async fn update_public_with_organization_promoter() {
        base::venues::update_with_organization(Roles::Promoter, false, false).await;
    }
    #[actix_rt::test]
    async fn update_public_with_organization_promoter_read_only() {
        base::venues::update_with_organization(Roles::PromoterReadOnly, false, false).await;
    }
    #[actix_rt::test]
    async fn update_public_with_organization_org_admin() {
        base::venues::update_with_organization(Roles::OrgAdmin, false, false).await;
    }
    #[actix_rt::test]
    async fn update_public_with_organization_box_office() {
        base::venues::update_with_organization(Roles::OrgBoxOffice, false, false).await;
    }
}

#[cfg(test)]
mod show_from_organizations_tests {
    use super::*;
    #[actix_rt::test]
    async fn show_from_organizations_org_member() {
        base::venues::show_from_organizations(Some(Roles::OrgMember), true).await;
    }
    #[actix_rt::test]
    async fn show_from_organizations_admin() {
        base::venues::show_from_organizations(Some(Roles::Admin), true).await;
    }
    #[actix_rt::test]
    async fn show_from_organizations_user() {
        base::venues::show_from_organizations(Some(Roles::User), true).await;
    }
    #[actix_rt::test]
    async fn show_from_organizations_org_owner() {
        base::venues::show_from_organizations(Some(Roles::OrgOwner), true).await;
    }
    #[actix_rt::test]
    async fn show_from_organizations_no_user() {
        base::venues::show_from_organizations(None, true).await;
    }
    #[actix_rt::test]
    async fn show_from_organizations_door_person() {
        base::venues::show_from_organizations(Some(Roles::DoorPerson), true).await;
    }
    #[actix_rt::test]
    async fn show_from_organizations_promoter() {
        base::venues::show_from_organizations(Some(Roles::Promoter), true).await;
    }
    #[actix_rt::test]
    async fn show_from_organizations_promoter_read_only() {
        base::venues::show_from_organizations(Some(Roles::PromoterReadOnly), true).await;
    }
    #[actix_rt::test]
    async fn show_from_organizations_org_admin() {
        base::venues::show_from_organizations(Some(Roles::OrgAdmin), true).await;
    }
    #[actix_rt::test]
    async fn show_from_organizations_box_office() {
        base::venues::show_from_organizations(Some(Roles::OrgBoxOffice), true).await;
    }
}

#[actix_rt::test]
pub async fn show_from_organizations_private_venue_same_org() {
    let database = TestDatabase::new();
    let user = database.create_user().finish();
    let organization = database
        .create_organization()
        .with_member(&user, Roles::OrgMember)
        .finish();
    let venue = database.create_venue().with_name("Venue 1".to_string()).finish();
    let venue2 = database
        .create_venue()
        .with_name("Venue 2".to_string())
        .make_private()
        .finish();
    venue
        .add_to_organization(organization.id, database.connection.get())
        .unwrap();
    venue2
        .add_to_organization(organization.id, database.connection.get())
        .unwrap();

    let user2 = database.create_user().finish();
    let auth_user = support::create_auth_user_from_user(&user2, Roles::OrgOwner, Some(&organization), &database);

    let all_venues = vec![venue, venue2];
    let wrapped_expected_venues = Payload {
        data: all_venues,
        paging: Paging {
            page: 0,
            limit: 100,
            sort: "".to_string(),
            dir: SortingDir::Asc,
            total: 2,
            tags: HashMap::new(),
        },
    };
    let expected_json = serde_json::to_string(&wrapped_expected_venues).unwrap();

    let test_request = TestRequest::create();

    let mut path = Path::<PathParameters>::extract(&test_request.request).await.unwrap();
    path.id = organization.id;

    let test_request = TestRequest::create_with_uri(&format!("/limits?"));
    let query_parameters = Query::<PagingParameters>::extract(&test_request.request).await.unwrap();
    let response: HttpResponse = venues::show_from_organizations((
        database.connection.into(),
        path,
        query_parameters,
        OptionalUser(Some(auth_user)),
    ))
    .await
    .into();

    assert_eq!(response.status(), StatusCode::OK);
    let body = support::unwrap_body_to_string(&response).unwrap();
    assert_eq!(expected_json, body);
}
