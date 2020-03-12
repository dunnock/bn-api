use crate::functional::base;
use crate::support;
use crate::support::database::TestDatabase;
use crate::support::test_request::TestRequest;
use actix_web::{
    http::StatusCode,
    web::{Path, Query},
    FromRequest, HttpResponse,
};
use bigneon_api::controllers::organization_invites::{self, InviteResponseQuery};
use bigneon_api::extractors::OptionalUser;
use bigneon_api::models::OrganizationInvitePathParameters;
use bigneon_db::models::*;

#[cfg(test)]
mod create_tests {
    use super::*;
    #[actix_rt::test]
    async fn create_org_member() {
        base::organization_invites::create(Roles::OrgMember, false).await;
    }
    #[actix_rt::test]
    async fn create_admin() {
        base::organization_invites::create(Roles::Admin, true).await;
    }
    #[actix_rt::test]
    async fn create_user() {
        base::organization_invites::create(Roles::User, false).await;
    }
    #[actix_rt::test]
    async fn create_org_owner() {
        base::organization_invites::create(Roles::OrgOwner, true).await;
    }
    #[actix_rt::test]
    async fn create_door_person() {
        base::organization_invites::create(Roles::DoorPerson, false).await;
    }
    #[actix_rt::test]
    async fn create_promoter() {
        base::organization_invites::create(Roles::Promoter, false).await;
    }
    #[actix_rt::test]
    async fn create_promoter_read_only() {
        base::organization_invites::create(Roles::PromoterReadOnly, false).await;
    }
    #[actix_rt::test]
    async fn create_org_admin() {
        base::organization_invites::create(Roles::OrgAdmin, true).await;
    }
    #[actix_rt::test]
    async fn create_box_office() {
        base::organization_invites::create(Roles::OrgBoxOffice, false).await;
    }
}

#[cfg(test)]
mod index_tests {
    use super::*;
    #[actix_rt::test]
    async fn index_org_member() {
        base::organization_invites::index(Roles::OrgMember, false).await;
    }
    #[actix_rt::test]
    async fn index_admin() {
        base::organization_invites::index(Roles::Admin, true).await;
    }
    #[actix_rt::test]
    async fn index_user() {
        base::organization_invites::index(Roles::User, false).await;
    }
    #[actix_rt::test]
    async fn index_org_owner() {
        base::organization_invites::index(Roles::OrgOwner, true).await;
    }
    #[actix_rt::test]
    async fn index_door_person() {
        base::organization_invites::index(Roles::DoorPerson, false).await;
    }
    #[actix_rt::test]
    async fn index_promoter() {
        base::organization_invites::index(Roles::Promoter, false).await;
    }
    #[actix_rt::test]
    async fn index_promoter_read_only() {
        base::organization_invites::index(Roles::PromoterReadOnly, false).await;
    }
    #[actix_rt::test]
    async fn index_org_admin() {
        base::organization_invites::index(Roles::OrgAdmin, true).await;
    }
    #[actix_rt::test]
    async fn index_box_office() {
        base::organization_invites::index(Roles::OrgBoxOffice, false).await;
    }
}

#[cfg(test)]
mod destroy_tests {
    use super::*;
    #[actix_rt::test]
    async fn destroy_org_member() {
        base::organization_invites::destroy(Roles::OrgMember, false).await;
    }
    #[actix_rt::test]
    async fn destroy_admin() {
        base::organization_invites::destroy(Roles::Admin, true).await;
    }
    #[actix_rt::test]
    async fn destroy_user() {
        base::organization_invites::destroy(Roles::User, false).await;
    }
    #[actix_rt::test]
    async fn destroy_org_owner() {
        base::organization_invites::destroy(Roles::OrgOwner, true).await;
    }
    #[actix_rt::test]
    async fn destroy_door_person() {
        base::organization_invites::destroy(Roles::DoorPerson, false).await;
    }
    #[actix_rt::test]
    async fn destroy_promoter() {
        base::organization_invites::destroy(Roles::Promoter, false).await;
    }
    #[actix_rt::test]
    async fn destroy_promoter_read_only() {
        base::organization_invites::destroy(Roles::PromoterReadOnly, false).await;
    }
    #[actix_rt::test]
    async fn destroy_org_admin() {
        base::organization_invites::destroy(Roles::OrgAdmin, true).await;
    }
    #[actix_rt::test]
    async fn destroy_box_office() {
        base::organization_invites::destroy(Roles::OrgBoxOffice, false).await;
    }
}

#[cfg(test)]
mod create_for_new_user_tests {
    use super::*;
    #[actix_rt::test]
    async fn create_for_new_user_org_member() {
        base::organization_invites::create_for_new_user(Roles::OrgMember, false).await;
    }
    #[actix_rt::test]
    async fn create_for_new_user_admin() {
        base::organization_invites::create_for_new_user(Roles::Admin, true).await;
    }
    #[actix_rt::test]
    async fn create_for_new_user_user() {
        base::organization_invites::create_for_new_user(Roles::User, false).await;
    }
    #[actix_rt::test]
    async fn create_for_new_user_org_owner() {
        base::organization_invites::create_for_new_user(Roles::OrgOwner, true).await;
    }
    #[actix_rt::test]
    async fn create_for_new_user_door_person() {
        base::organization_invites::create_for_new_user(Roles::DoorPerson, false).await;
    }
    #[actix_rt::test]
    async fn create_for_new_user_promoter() {
        base::organization_invites::create_for_new_user(Roles::Promoter, false).await;
    }
    #[actix_rt::test]
    async fn create_for_new_user_promoter_read_only() {
        base::organization_invites::create_for_new_user(Roles::PromoterReadOnly, false).await;
    }
    #[actix_rt::test]
    async fn create_for_new_user_org_admin() {
        base::organization_invites::create_for_new_user(Roles::OrgAdmin, true).await;
    }
    #[actix_rt::test]
    async fn create_for_new_user_box_office() {
        base::organization_invites::create_for_new_user(Roles::OrgBoxOffice, false).await;
    }
}

#[cfg(test)]
mod accept_tests {
    use super::*;
    #[actix_rt::test]
    async fn accept_org_member() {
        base::organization_invites::accept_invite_status_of_invite(Roles::OrgMember, true).await;
    }
    #[actix_rt::test]
    async fn accept_admin() {
        base::organization_invites::accept_invite_status_of_invite(Roles::Admin, true).await;
    }
    #[actix_rt::test]
    async fn accept_user() {
        base::organization_invites::accept_invite_status_of_invite(Roles::User, true).await;
    }
    #[actix_rt::test]
    async fn accept_org_owner() {
        base::organization_invites::accept_invite_status_of_invite(Roles::OrgOwner, true).await;
    }
    #[actix_rt::test]
    async fn accept_door_person() {
        base::organization_invites::accept_invite_status_of_invite(Roles::DoorPerson, true).await;
    }
    #[actix_rt::test]
    async fn accept_promoter() {
        base::organization_invites::accept_invite_status_of_invite(Roles::Promoter, true).await;
    }
    #[actix_rt::test]
    async fn accept_promoter_read_only() {
        base::organization_invites::accept_invite_status_of_invite(Roles::PromoterReadOnly, true).await;
    }
    #[actix_rt::test]
    async fn accept_org_admin() {
        base::organization_invites::accept_invite_status_of_invite(Roles::OrgAdmin, true).await;
    }
    #[actix_rt::test]
    async fn accept_box_office() {
        base::organization_invites::accept_invite_status_of_invite(Roles::OrgBoxOffice, true).await;
    }
}

#[actix_rt::test]
pub async fn accept_invite_for_other_email_succeeds() {
    let database = TestDatabase::new();
    let user = database.create_user().finish();
    let organization = database.create_organization().finish();
    let auth_user = support::create_auth_user_from_user(&user, Roles::OrgAdmin, Some(&organization), &database);
    database.create_user().finish();

    let email = "different@email.com".to_string();
    let invite = database
        .create_organization_invite()
        .with_org(&organization)
        .with_invitee(&user)
        .with_email(&email)
        .with_security_token(None)
        .finish();

    OrganizationInvite::find_by_token(invite.security_token.unwrap(), database.connection.get()).unwrap();

    let test_request = TestRequest::create_with_uri(
        format!(
            "/accept_invite?security_token={}",
            &invite.security_token.unwrap().to_string()
        )
        .as_str(),
    );
    let parameters = Query::<InviteResponseQuery>::extract(&test_request.request)
        .await
        .unwrap();

    let response: HttpResponse =
        organization_invites::accept_request((database.connection.into(), parameters, OptionalUser(Some(auth_user))))
            .await
            .into();
    assert_eq!(response.status(), StatusCode::OK);
}

#[actix_rt::test]
pub async fn accept_invite_for_user_id_succeeds() {
    let database = TestDatabase::new();
    let user = database.create_user().finish();
    let user2 = database.create_user().finish();
    let organization = database.create_organization().finish();
    let auth_user = support::create_auth_user_from_user(&user, Roles::Promoter, Some(&organization), &database);
    database.create_user().finish();

    let invite = database
        .create_organization_invite()
        .with_org(&organization)
        .with_invitee(&user2)
        .link_to_user(&user)
        .with_role(Roles::PromoterReadOnly)
        .with_security_token(None)
        .finish();

    OrganizationInvite::find_by_token(invite.security_token.unwrap(), database.connection.get()).unwrap();

    let test_request = TestRequest::create_with_uri(
        format!(
            "/accept_invite?security_token={}",
            &invite.security_token.unwrap().to_string()
        )
        .as_str(),
    );
    let parameters = Query::<InviteResponseQuery>::extract(&test_request.request)
        .await
        .unwrap();

    let response: HttpResponse =
        organization_invites::accept_request((database.connection.into(), parameters, OptionalUser(Some(auth_user))))
            .await
            .into();
    assert_eq!(response.status(), StatusCode::OK);
}

#[actix_rt::test]
pub async fn accept_invite_for_other_user_id_fails() {
    let database = TestDatabase::new();
    let user = database.create_user().finish();
    let user2 = database.create_user().finish();
    let organization = database.create_organization().finish();
    let auth_user = support::create_auth_user_from_user(&user, Roles::OrgAdmin, Some(&organization), &database);
    database.create_user().finish();

    let invite = database
        .create_organization_invite()
        .with_org(&organization)
        .with_invitee(&user)
        .link_to_user(&user2)
        .with_security_token(None)
        .finish();

    OrganizationInvite::find_by_token(invite.security_token.unwrap(), database.connection.get()).unwrap();

    let test_request = TestRequest::create_with_uri(
        format!(
            "/accept_invite?security_token={}",
            &invite.security_token.unwrap().to_string()
        )
        .as_str(),
    );
    let parameters = Query::<InviteResponseQuery>::extract(&test_request.request)
        .await
        .unwrap();

    let response: HttpResponse =
        organization_invites::accept_request((database.connection.into(), parameters, OptionalUser(Some(auth_user))))
            .await
            .into();
    support::expects_unauthorized(&response);
}

#[actix_rt::test]
async fn destroy_owner_role_invite_as_organization_member_fails() {
    let database = TestDatabase::new();
    let user = database.create_user().finish();
    let organization = database.create_organization().finish();
    let auth_user = support::create_auth_user_from_user(&user, Roles::OrgAdmin, Some(&organization), &database);

    let invite = database
        .create_organization_invite()
        .with_role(Roles::OrgOwner)
        .with_org(&organization)
        .with_invitee(&user)
        .finish();

    let test_request = TestRequest::create_with_uri_custom_params("/", vec!["id", "invite_id"]);
    let mut path = Path::<OrganizationInvitePathParameters>::extract(&test_request.request)
        .await
        .unwrap();
    path.id = organization.id;
    path.invite_id = invite.id;

    let response: HttpResponse = organization_invites::destroy((database.connection.clone().into(), path, auth_user))
        .await
        .into();

    support::expects_unauthorized(&response);
}
