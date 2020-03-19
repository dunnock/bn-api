use crate::auth::user::User as AuthUser;
use crate::communications::mailers;
use crate::database::Connection;
use crate::errors::*;
use crate::extractors::*;
use crate::helpers::application;
use crate::models::{OrganizationInvitePathParameters, PathParameters, WebPayload};
use crate::server::AppState;
use actix_web::{
    http::StatusCode,
    web::{Data, Path, Query},
    HttpResponse,
};
use db::models::*;
use db::utils::errors::DatabaseError;
use db::utils::errors::Optional;
use diesel::pg::PgConnection;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct Info {
    pub token: Uuid,
    pub user_id: Uuid,
}

#[derive(Deserialize)]
pub struct InviteResponseQuery {
    pub security_token: Uuid,
}

#[derive(Deserialize)]
pub struct NewOrgInviteRequest {
    pub user_email: String,
    pub roles: Vec<Roles>,
    pub event_ids: Option<Vec<Uuid>>,
}
pub async fn create_for_event(
    (state, connection, new_org_invite, path, auth_user): (
        Data<AppState>,
        Connection,
        Json<NewOrgInviteRequest>,
        Path<PathParameters>,
        AuthUser,
    ),
) -> Result<HttpResponse, ApiError> {
    let connection = connection.get();
    let organization = Event::find(path.id, connection)?.organization(connection)?;
    let mut invite = new_org_invite.into_inner();
    invite.event_ids = Some(vec![path.id]);
    create_invite(state, connection, invite, &organization, auth_user)
}

pub async fn create(
    (state, connection, new_org_invite, path, auth_user): (
        Data<AppState>,
        Connection,
        Json<NewOrgInviteRequest>,
        Path<PathParameters>,
        AuthUser,
    ),
) -> Result<HttpResponse, ApiError> {
    let connection = connection.get();
    let organization = Organization::find(path.id, connection)?;
    create_invite(state, connection, new_org_invite.into_inner(), &organization, auth_user)
}

fn create_invite(
    state: Data<AppState>,
    connection: &PgConnection,
    new_org_invite: NewOrgInviteRequest,
    organization: &Organization,
    auth_user: AuthUser,
) -> Result<HttpResponse, ApiError> {
    for role in &new_org_invite.roles {
        match role {
            &Roles::OrgOwner => {
                auth_user.requires_scope_for_organization(Scopes::OrgAdmin, &organization, connection)?
            }
            &Roles::OrgAdmin => {
                auth_user.requires_scope_for_organization(Scopes::OrgAdminUsers, &organization, connection)?
            }
            &Roles::PrismIntegration => {
                auth_user.requires_scope_for_organization(Scopes::OrgAdminUsers, &organization, connection)?
            }
            &Roles::OrgMember => {
                auth_user.requires_scope_for_organization(Scopes::OrgUsers, &organization, connection)?
            }
            &Roles::DoorPerson => {
                auth_user.requires_scope_for_organization(Scopes::OrgUsers, &organization, connection)?
            }
            &Roles::OrgBoxOffice => {
                auth_user.requires_scope_for_organization(Scopes::OrgUsers, &organization, connection)?
            }
            &Roles::Promoter => {
                auth_user.requires_scope_for_organization(Scopes::OrgUsers, &organization, connection)?
            }
            &Roles::PromoterReadOnly => {
                auth_user.requires_scope_for_organization(Scopes::OrgUsers, &organization, connection)?
            }
            _ => DatabaseError::validation_error("role", "Role not valid")?,
        }
    }

    let mut invite: NewOrganizationInvite;
    let recipient: String;
    let user_id: Option<Uuid>;

    match User::find_by_email(&new_org_invite.user_email, false, connection).optional()? {
        Some(user) => {
            recipient = user.full_name();
            user_id = Some(user.id);
        }
        None => {
            recipient = "New user".to_string();
            user_id = None;
        }
    };

    //If an active invite exists for this email then first expire it before issuing the new invite.
    if let Some(event_ids) = &new_org_invite.event_ids {
        // If this invite is related to event access check to see if this event already has an invite
        if let Some(mut i) = OrganizationInvite::find_active_organization_invite_for_email(
            &new_org_invite.user_email,
            &organization,
            Some(event_ids),
            connection,
        )? {
            i.change_invite_status(0, connection)?;
        }
    } else {
        // For invites giving full organization access only check if this user has an active invite for the organization
        if let Some(mut i) = OrganizationInvite::find_active_organization_invite_for_email(
            &new_org_invite.user_email,
            &organization,
            None,
            connection,
        )? {
            i.change_invite_status(0, connection)?;
        }
    }

    invite = OrganizationInvite::create(
        organization.id,
        auth_user.id(),
        new_org_invite.user_email.as_str(),
        user_id,
        new_org_invite.roles.clone(),
        new_org_invite.event_ids.clone(),
    );

    let mut invite = invite.commit(connection)?;
    let organization = Organization::find(invite.organization_id, connection)?;

    // If the user already exists for organization, accept the invite immediately, otherwise send an email
    let mut mail_invite = true;
    if let Some(user_id) = user_id {
        let organization_user = OrganizationUser::find_by_user_id(user_id, organization.id, connection).optional()?;
        if organization_user.is_some() {
            mail_invite = false;
            accept_invite(user_id, &mut invite, &connection)?;
        }
    }

    if mail_invite {
        mailers::organization_invites::invite_user_to_organization_email(
            &state.config,
            &invite,
            &organization,
            &recipient,
            connection,
        )?;
    }

    Ok(HttpResponse::Created().json(invite))
}

pub async fn index(
    (connection, path, query_parameters, auth_user): (
        Connection,
        Path<PathParameters>,
        Query<PagingParameters>,
        AuthUser,
    ),
) -> Result<WebPayload<DisplayInvite>, ApiError> {
    let connection = connection.get();
    let organization = Organization::find(path.id, connection)?;
    auth_user.requires_scope_for_organization(Scopes::OrgUsers, &organization, connection)?;

    let payload = OrganizationInvite::find_pending_by_organization_paged(
        path.id,
        query_parameters.page(),
        query_parameters.limit(),
        connection,
    )?;

    Ok(WebPayload::new(StatusCode::OK, payload))
}

pub async fn destroy(
    (connection, path, auth_user): (Connection, Path<OrganizationInvitePathParameters>, AuthUser),
) -> Result<HttpResponse, ApiError> {
    let connection = connection.get();
    let invite = OrganizationInvite::find(path.invite_id, connection)?;
    let organization = invite.organization(connection)?;

    // Level of access dependent on scope of the invited member
    for role in &invite.roles {
        match role {
            &Roles::OrgOwner => {
                auth_user.requires_scope_for_organization(Scopes::OrgAdmin, &organization, connection)?
            }
            &Roles::OrgAdmin => {
                auth_user.requires_scope_for_organization(Scopes::OrgAdminUsers, &organization, connection)?
            }
            &Roles::OrgMember => {
                auth_user.requires_scope_for_organization(Scopes::OrgUsers, &organization, connection)?
            }
            &Roles::PrismIntegration => {
                auth_user.requires_scope_for_organization(Scopes::OrgAdminUsers, &organization, connection)?
            }
            &Roles::DoorPerson => {
                auth_user.requires_scope_for_organization(Scopes::OrgUsers, &organization, connection)?
            }
            &Roles::OrgBoxOffice => {
                auth_user.requires_scope_for_organization(Scopes::OrgUsers, &organization, connection)?
            }
            &Roles::Promoter => {
                auth_user.requires_scope_for_organization(Scopes::OrgUsers, &organization, connection)?
            }
            &Roles::PromoterReadOnly => {
                auth_user.requires_scope_for_organization(Scopes::OrgUsers, &organization, connection)?
            }
            // Should not happen but if it ever did allow admin to destroy record
            _ => auth_user.requires_scope_for_organization(Scopes::OrgAdmin, &organization, connection)?,
        }
    }

    invite.destroy(connection)?;
    Ok(HttpResponse::Ok().json(json!({})))
}

pub async fn view((connection, path): (Connection, Path<PathParameters>)) -> Result<HttpResponse, ApiError> {
    // TODO: Change /{id} to /?token={} in routing and client apps.
    // Until then, just remember that the id passed in is actually the token
    let connection = connection.get();
    let invite_details = OrganizationInvite::get_invite_display(&path.id, connection)?;
    Ok(HttpResponse::Ok().json(json!(invite_details)))
}

pub async fn accept_request(
    (connection, query, user): (Connection, Query<InviteResponseQuery>, OptionalUser),
) -> Result<HttpResponse, ApiError> {
    let query_struct = query.into_inner();
    let connection = connection.get();
    let mut invite_details = OrganizationInvite::find_by_token(query_struct.security_token, connection)?;
    //Check that the user is logged in, that if the invite has a user_id associated with it that it is the currently logged in user
    match user.into_inner() {
        Some(u) => {
            let mut valid_for_acceptance = true;
            if let Some(user_id) = invite_details.user_id {
                // If the user_id was provided confirm that the current user is the accepting user
                valid_for_acceptance = user_id == u.id();
            };

            if valid_for_acceptance {
                accept_invite(u.id(), &mut invite_details, &connection)?;
            } else {
                return application::unauthorized(Some(u), None);
            }
        }
        None => return application::unauthorized(None, None),
    }
    Ok(HttpResponse::Ok().finish())
}

fn accept_invite(user_id: Uuid, invite: &mut OrganizationInvite, connection: &PgConnection) -> Result<(), ApiError> {
    invite.change_invite_status(1, connection)?;
    let organization = Organization::find(invite.organization_id, connection)?;
    let organization_user =
        organization.add_user(user_id, invite.roles.clone(), invite.event_ids.clone(), connection)?;

    // Check for any additional pending invites for this organization and accept them
    if organization_user.is_event_user() {
        for mut related_invite in OrganizationInvite::find_all_active_organization_invites_by_email(
            &invite.user_email,
            &organization,
            connection,
        )? {
            if related_invite.is_event_user() {
                organization.add_user(
                    user_id,
                    related_invite.roles.clone(),
                    related_invite.event_ids.clone(),
                    connection,
                )?;
                related_invite.change_invite_status(1, connection)?;
            }
        }
    }

    Ok(())
}
