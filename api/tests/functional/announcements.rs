use actix_web::{http::StatusCode, FromRequest, HttpResponse, Path};
use bigneon_api::controllers::announcements::{self, *};
use bigneon_api::extractors::*;
use bigneon_api::models::*;
use bigneon_db::models::*;
use bigneon_db::utils::dates;
use diesel;
use diesel::query_dsl::RunQueryDsl;
use diesel::sql_types;
use functional::base;
use support;
use support::database::TestDatabase;
use support::test_request::TestRequest;

#[cfg(test)]
mod index_tests {
    use super::*;
    #[test]
    fn index_org_member() {
        base::announcements::index(Roles::OrgMember, false);
    }
    #[test]
    fn index_admin() {
        base::announcements::index(Roles::Admin, true);
    }
    #[test]
    fn index_super() {
        base::announcements::index(Roles::Super, true);
    }
    #[test]
    fn index_user() {
        base::announcements::index(Roles::User, false);
    }
    #[test]
    fn index_org_owner() {
        base::announcements::index(Roles::OrgOwner, false);
    }
    #[test]
    fn index_door_person() {
        base::announcements::index(Roles::DoorPerson, false);
    }
    #[test]
    fn index_promoter() {
        base::announcements::index(Roles::Promoter, false);
    }
    #[test]
    fn index_promoter_read_only() {
        base::announcements::index(Roles::PromoterReadOnly, false);
    }
    #[test]
    fn index_org_admin() {
        base::announcements::index(Roles::OrgAdmin, false);
    }
    #[test]
    fn index_box_office() {
        base::announcements::index(Roles::OrgBoxOffice, false);
    }
}

#[cfg(test)]
mod show_tests {
    use super::*;
    #[test]
    fn show_org_member() {
        base::announcements::show(Roles::OrgMember, false);
    }
    #[test]
    fn show_admin() {
        base::announcements::show(Roles::Admin, true);
    }
    #[test]
    fn show_super() {
        base::announcements::show(Roles::Super, true);
    }
    #[test]
    fn show_user() {
        base::announcements::show(Roles::User, false);
    }
    #[test]
    fn show_org_owner() {
        base::announcements::show(Roles::OrgOwner, false);
    }
    #[test]
    fn show_door_person() {
        base::announcements::show(Roles::DoorPerson, false);
    }
    #[test]
    fn show_promoter() {
        base::announcements::show(Roles::Promoter, false);
    }
    #[test]
    fn show_promoter_read_only() {
        base::announcements::show(Roles::PromoterReadOnly, false);
    }
    #[test]
    fn show_org_admin() {
        base::announcements::show(Roles::OrgAdmin, false);
    }
    #[test]
    fn show_box_office() {
        base::announcements::show(Roles::OrgBoxOffice, false);
    }
}

#[cfg(test)]
mod create_tests {
    use super::*;
    #[test]
    fn create_org_member() {
        base::announcements::create(Roles::OrgMember, false);
    }
    #[test]
    fn create_admin() {
        base::announcements::create(Roles::Admin, true);
    }
    #[test]
    fn create_super() {
        base::announcements::create(Roles::Super, true);
    }
    #[test]
    fn create_user() {
        base::announcements::create(Roles::User, false);
    }
    #[test]
    fn create_org_owner() {
        base::announcements::create(Roles::OrgOwner, false);
    }
    #[test]
    fn create_door_person() {
        base::announcements::create(Roles::DoorPerson, false);
    }
    #[test]
    fn create_promoter() {
        base::announcements::create(Roles::Promoter, false);
    }
    #[test]
    fn create_promoter_read_only() {
        base::announcements::create(Roles::PromoterReadOnly, false);
    }
    #[test]
    fn create_org_admin() {
        base::announcements::create(Roles::OrgAdmin, false);
    }
    #[test]
    fn create_box_office() {
        base::announcements::create(Roles::OrgBoxOffice, false);
    }
}

#[cfg(test)]
mod update_tests {
    use super::*;
    #[test]
    fn update_org_member() {
        base::announcements::update(Roles::OrgMember, false);
    }
    #[test]
    fn update_admin() {
        base::announcements::update(Roles::Admin, true);
    }
    #[test]
    fn update_super() {
        base::announcements::update(Roles::Super, true);
    }
    #[test]
    fn update_user() {
        base::announcements::update(Roles::User, false);
    }
    #[test]
    fn update_org_owner() {
        base::announcements::update(Roles::OrgOwner, false);
    }
    #[test]
    fn update_door_person() {
        base::announcements::update(Roles::DoorPerson, false);
    }
    #[test]
    fn update_promoter() {
        base::announcements::update(Roles::Promoter, false);
    }
    #[test]
    fn update_promoter_read_only() {
        base::announcements::update(Roles::PromoterReadOnly, false);
    }
    #[test]
    fn update_org_admin() {
        base::announcements::update(Roles::OrgAdmin, false);
    }
    #[test]
    fn update_box_office() {
        base::announcements::update(Roles::OrgBoxOffice, false);
    }
}

#[cfg(test)]
mod destroy_tests {
    use super::*;
    #[test]
    fn destroy_org_member() {
        base::announcements::destroy(Roles::OrgMember, false);
    }
    #[test]
    fn destroy_admin() {
        base::announcements::destroy(Roles::Admin, true);
    }
    #[test]
    fn destroy_super() {
        base::announcements::destroy(Roles::Super, true);
    }
    #[test]
    fn destroy_user() {
        base::announcements::destroy(Roles::User, false);
    }
    #[test]
    fn destroy_org_owner() {
        base::announcements::destroy(Roles::OrgOwner, false);
    }
    #[test]
    fn destroy_door_person() {
        base::announcements::destroy(Roles::DoorPerson, false);
    }
    #[test]
    fn destroy_promoter() {
        base::announcements::destroy(Roles::Promoter, false);
    }
    #[test]
    fn destroy_promoter_read_only() {
        base::announcements::destroy(Roles::PromoterReadOnly, false);
    }
    #[test]
    fn destroy_org_admin() {
        base::announcements::destroy(Roles::OrgAdmin, false);
    }
    #[test]
    fn destroy_box_office() {
        base::announcements::destroy(Roles::OrgBoxOffice, false);
    }
}

#[test]
fn show_from_organization() {
    let database = TestDatabase::new();
    let connection = database.connection.get();
    let organization = database.create_organization().finish();
    let organization2 = database.create_organization().finish();
    let announcement = database.create_announcement().finish();
    diesel::sql_query(
        r#"
        UPDATE announcements
        SET created_at = $1
        WHERE id = $2;
        "#,
    )
    .bind::<sql_types::Timestamp, _>(dates::now().add_hours(-1).finish())
    .bind::<sql_types::Uuid, _>(announcement.id)
    .execute(connection)
    .unwrap();
    let announcement = Announcement::find(announcement.id, false, connection).unwrap();
    let announcement2 = database.create_announcement().with_organization(&organization).finish();
    let _announcement3 = database
        .create_announcement()
        .with_organization(&organization2)
        .finish();

    let expected_announcements = vec![announcement, announcement2];
    let expected_json = serde_json::to_string(&expected_announcements).unwrap();

    let test_request = TestRequest::create();
    let mut path = Path::<PathParameters>::extract(&test_request.request).unwrap();
    path.id = organization.id;

    let user = support::create_auth_user(Roles::OrgMember, Some(&organization), &database);
    let response: HttpResponse = announcements::show_from_organization((database.connection.into(), path, user)).into();
    assert_eq!(response.status(), StatusCode::OK);
    let body = support::unwrap_body_to_string(&response).unwrap();
    assert_eq!(body, expected_json);
}

#[test]
fn show_from_organization_with_engagements_hiding() {
    let database = TestDatabase::new();
    let connection = database.connection.get();
    let organization = database.create_organization().with_event_fee().with_fees().finish();
    let user = support::create_auth_user(Roles::OrgMember, Some(&organization), &database);
    let announcement = database.create_announcement().finish();
    diesel::sql_query(
        r#"
        UPDATE announcements
        SET created_at = $1
        WHERE id = $2;
        "#,
    )
    .bind::<sql_types::Timestamp, _>(dates::now().add_hours(-1).finish())
    .bind::<sql_types::Uuid, _>(announcement.id)
    .execute(connection)
    .unwrap();
    let announcement = Announcement::find(announcement.id, false, connection).unwrap();
    let announcement2 = database.create_announcement().with_organization(&organization).finish();
    database
        .create_announcement_engagement()
        .with_announcement(&announcement2)
        .with_user(&user.user)
        .finish();

    let expected_announcements = vec![announcement];
    let expected_json = serde_json::to_string(&expected_announcements).unwrap();

    let test_request = TestRequest::create();
    let mut path = Path::<PathParameters>::extract(&test_request.request).unwrap();
    path.id = organization.id;

    let response: HttpResponse = announcements::show_from_organization((database.connection.into(), path, user)).into();
    assert_eq!(response.status(), StatusCode::OK);
    let body = support::unwrap_body_to_string(&response).unwrap();
    assert_eq!(body, expected_json);
}

#[test]
fn engage() {
    let database = TestDatabase::new();
    let connection = database.connection.get();
    let organization = database.create_organization().with_event_fee().with_fees().finish();
    let announcement = database.create_announcement().finish();
    let test_request = TestRequest::create();
    let mut path = Path::<PathParameters>::extract(&test_request.request).unwrap();
    path.id = announcement.id;

    let user = support::create_auth_user(Roles::OrgMember, Some(&organization), &database);
    let data = Json(EngagementData { action: None });
    assert!(AnnouncementEngagement::find_by_announcement_id_user_id(announcement.id, user.id(), connection).is_err());

    let response: HttpResponse =
        announcements::engage((database.connection.clone().into(), path, data, user.clone())).into();
    assert_eq!(response.status(), StatusCode::OK);
    assert!(AnnouncementEngagement::find_by_announcement_id_user_id(announcement.id, user.id(), connection).is_ok());
}
