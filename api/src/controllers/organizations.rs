use actix_web::{http::StatusCode, HttpResponse, web::{Path, Query, Data}};
use crate::auth::user::User;
use bigneon_db::models::*;
use chrono::NaiveDateTime;
use crate::db::{Connection, ReadonlyConnection};
use crate::errors::*;
use crate::extractors::*;
use crate::helpers::application;
use crate::models::WebPayload;
use crate::models::{OrganizationUserPathParameters, PathParameters};
use crate::server::AppState;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct AddUserRequest {
    pub user_id: Uuid,
    pub roles: Vec<Roles>,
    pub event_ids: Option<Vec<Uuid>>,
}

#[derive(Serialize, Deserialize)]
pub struct FeeScheduleWithRanges {
    pub id: Uuid,
    pub name: String,
    pub version: i16,
    pub created_at: NaiveDateTime,
    pub ranges: Vec<FeeScheduleRange>,
}

#[derive(Serialize, Deserialize)]
pub struct NewOrganizationRequest {
    pub name: String,
    pub client_event_fee_in_cents: Option<i64>,
    pub company_event_fee_in_cents: Option<i64>,
    #[serde(default, deserialize_with = "deserialize_unless_blank")]
    pub address: Option<String>,
    #[serde(default, deserialize_with = "deserialize_unless_blank")]
    pub city: Option<String>,
    #[serde(default, deserialize_with = "deserialize_unless_blank")]
    pub state: Option<String>,
    #[serde(default, deserialize_with = "deserialize_unless_blank")]
    pub country: Option<String>,
    #[serde(default, deserialize_with = "deserialize_unless_blank")]
    pub postal_code: Option<String>,
    #[serde(default, deserialize_with = "deserialize_unless_blank")]
    pub phone: Option<String>,
    #[serde(default, deserialize_with = "deserialize_unless_blank")]
    pub sendgrid_api_key: Option<String>,
    #[serde(default, deserialize_with = "deserialize_unless_blank")]
    pub google_ga_key: Option<String>,
    #[serde(default, deserialize_with = "deserialize_unless_blank")]
    pub facebook_pixel_key: Option<String>,
    pub allowed_payment_providers: Option<Vec<PaymentProviders>>,
    #[serde(default, deserialize_with = "deserialize_unless_blank")]
    pub timezone: Option<String>,
    #[serde(default, deserialize_with = "deserialize_unless_blank")]
    pub globee_api_key: Option<String>,
    pub cc_fee_percent: Option<f32>,
    pub max_instances_per_ticket_type: Option<i64>,
    pub settlement_type: Option<SettlementTypes>,
}

#[derive(Serialize, Deserialize)]
pub struct NewFeeScheduleRequest {
    pub name: String,
    pub ranges: Vec<NewFeeScheduleRange>,
}

pub fn index(
    (connection, query_parameters, user): (Connection, Query<PagingParameters>, User),
) -> Result<HttpResponse, BigNeonError> {
    if user.has_scope(Scopes::OrgAdmin)? {
        return index_for_all_orgs((connection, query_parameters, user));
    }

    //TODO remap query to use paging info
    let organizations = Organization::all_linked_to_user(user.id(), connection.get())?;

    Ok(HttpResponse::Ok().json(&Payload::from_data(
        organizations,
        query_parameters.page(),
        query_parameters.limit(),
        None,
    )))
}

pub fn index_for_all_orgs(
    (connection, query_parameters, user): (Connection, Query<PagingParameters>, User),
) -> Result<HttpResponse, BigNeonError> {
    user.requires_scope(Scopes::OrgAdmin)?;
    let connection = connection.get();
    let organizations = Organization::all(connection)?;

    Ok(HttpResponse::Ok().json(&Payload::from_data(
        organizations,
        query_parameters.page(),
        query_parameters.limit(),
        None,
    )))
}

pub fn show(
    (state, connection, parameters, user): (Data<AppState>, Connection, Path<PathParameters>, User),
) -> Result<HttpResponse, BigNeonError> {
    let connection = connection.get();
    let mut organization = Organization::find(parameters.id, connection)?;
    user.requires_scope_for_organization(Scopes::OrgRead, &organization, connection)?;

    organization.decrypt(&state.config.api_keys_encryption_key)?;

    Ok(HttpResponse::Ok().json(&organization))
}

pub fn create(
    (state, connection, new_organization, user): (Data<AppState>, Connection, Json<NewOrganizationRequest>, User),
) -> Result<HttpResponse, BigNeonError> {
    user.requires_scope(Scopes::OrgAdmin)?;
    let connection = connection.get();

    let fee_schedule = FeeSchedule::create(
        Uuid::nil(),
        format!("{} default fees", new_organization.name),
        vec![NewFeeScheduleRange {
            min_price_in_cents: 0,
            company_fee_in_cents: 0,
            client_fee_in_cents: 0,
        }],
    )
    .commit(Some(user.id()), connection)?;

    let new_organization_with_fee_schedule = NewOrganization {
        name: new_organization.name.clone(),
        fee_schedule_id: fee_schedule.id,
        client_event_fee_in_cents: new_organization.client_event_fee_in_cents,
        company_event_fee_in_cents: new_organization.company_event_fee_in_cents,
        address: new_organization.address.clone(),
        city: new_organization.city.clone(),
        state: new_organization.state.clone(),
        country: new_organization.country.clone(),
        postal_code: new_organization.postal_code.clone(),
        phone: new_organization.phone.clone(),
        sendgrid_api_key: new_organization.sendgrid_api_key.clone(),
        google_ga_key: new_organization.google_ga_key.clone(),
        facebook_pixel_key: new_organization.facebook_pixel_key.clone(),
        allowed_payment_providers: new_organization.allowed_payment_providers.clone(),
        timezone: new_organization.timezone.clone(),
        cc_fee_percent: new_organization.cc_fee_percent.unwrap_or(0f32),
        globee_api_key: new_organization.globee_api_key.clone(),
        max_instances_per_ticket_type: Some(match new_organization.max_instances_per_ticket_type {
            Some(x) => x,
            None => state.config.max_instances_per_ticket_type,
        }),
        settlement_type: new_organization.settlement_type,
    };

    let mut organization = new_organization_with_fee_schedule.commit(
        state.config.settlement_period_in_days,
        &state.config.api_keys_encryption_key,
        Some(user.id()),
        connection,
    )?;

    organization.decrypt(&state.config.api_keys_encryption_key)?;

    Wallet::create_for_organization(organization.id, "Default".to_string(), connection)?;

    Ok(HttpResponse::Created().json(&organization))
}

pub fn update(
    (state, connection, parameters, organization_parameters, user): (
        Data<AppState>,
        Connection,
        Path<PathParameters>,
        Json<OrganizationEditableAttributes>,
        User,
    ),
) -> Result<HttpResponse, BigNeonError> {
    let conn = connection.get();
    let mut organization = Organization::find(parameters.id, conn)?;
    let organization_update = organization_parameters.into_inner();

    if organization_update.settlement_type.is_some() {
        user.requires_scope_for_organization(Scopes::OrgModifySettlementType, &organization, conn)?;
    } else if organization_update.max_instances_per_ticket_type.is_some() {
        user.requires_scope_for_organization(Scopes::OrgAdmin, &organization, conn)?;
    } else {
        user.requires_scope_for_organization(Scopes::OrgWrite, &organization, conn)?;
    }

    let mut updated_organization = organization.update(
        organization_update,
        state.config.settlement_period_in_days,
        &state.config.api_keys_encryption_key,
        conn,
    )?;

    organization.decrypt(&state.config.api_keys_encryption_key)?;
    updated_organization.decrypt(&state.config.api_keys_encryption_key)?;

    Ok(HttpResponse::Ok().json(&updated_organization))
}

pub fn add_artist(
    (connection, parameters, new_artist, user): (Connection, Path<PathParameters>, Json<NewArtist>, User),
) -> Result<HttpResponse, BigNeonError> {
    let connection = connection.get();
    let organization = Organization::find(parameters.id, connection)?;
    user.requires_scope_for_organization(Scopes::OrgWrite, &organization, connection)?;

    let mut new_artist = new_artist.into_inner();
    new_artist.organization_id = Some(parameters.id);

    let artist = new_artist.commit(connection)?;
    Ok(HttpResponse::Created().json(&artist))
}

pub fn add_or_replace_user(
    (connection, path, json, user): (Connection, Path<PathParameters>, Json<AddUserRequest>, User),
) -> Result<HttpResponse, BigNeonError> {
    let connection = connection.get();
    let organization = Organization::find(path.id, connection)?;

    let req = json.into_inner();

    for role in req.roles.iter() {
        match role {
            Roles::OrgOwner => user.requires_scope_for_organization(Scopes::OrgAdmin, &organization, connection)?,
            Roles::OrgAdmin => {
                user.requires_scope_for_organization(Scopes::OrgAdminUsers, &organization, connection)?
            }
            Roles::PrismIntegration => {
                user.requires_scope_for_organization(Scopes::OrgAdminUsers, &organization, connection)?
            }
            Roles::OrgMember => user.requires_scope_for_organization(Scopes::OrgUsers, &organization, connection)?,
            Roles::DoorPerson => user.requires_scope_for_organization(Scopes::OrgUsers, &organization, connection)?,
            Roles::OrgBoxOffice => user.requires_scope_for_organization(Scopes::OrgUsers, &organization, connection)?,
            Roles::Promoter => user.requires_scope_for_organization(Scopes::OrgUsers, &organization, connection)?,
            Roles::PromoterReadOnly => {
                user.requires_scope_for_organization(Scopes::OrgUsers, &organization, connection)?
            }
            _ => return application::forbidden("Role is not allowed for this user"),
        };
    }

    organization.add_user(req.user_id, req.roles, req.event_ids.unwrap_or(Vec::new()), connection)?;

    Ok(HttpResponse::Created().finish())
}

pub fn remove_user(
    (connection, parameters, user): (Connection, Path<OrganizationUserPathParameters>, User),
) -> Result<HttpResponse, BigNeonError> {
    let connection = connection.get();
    let organization = Organization::find(parameters.id, connection)?;
    user.requires_scope_for_organization(Scopes::OrgUsers, &organization, connection)?;

    let organization = organization.remove_user(parameters.user_id, connection)?;
    Ok(HttpResponse::Ok().json(&organization))
}

pub fn list_organization_members(
    (connection, path_parameters, query_parameters, user): (
        Connection,
        Path<PathParameters>,
        Query<PagingParameters>,
        User,
    ),
) -> Result<WebPayload<DisplayOrganizationUser>, BigNeonError> {
    let connection = connection.get();
    //TODO refactor Organization::find to use limits as in PagingParameters
    let organization = Organization::find(path_parameters.id, connection)?;
    user.requires_scope_for_organization(Scopes::OrgRead, &organization, connection)?;

    let mut members: Vec<DisplayOrganizationUser> = organization
        .users(None, connection)?
        .into_iter()
        .map(|u| DisplayOrganizationUser {
            user_id: Some(u.1.id),
            first_name: u.1.first_name,
            last_name: u.1.last_name,
            email: u.1.email,
            roles: u.0.role,
            invite_or_member: "member".to_string(),
            invite_id: None,
        })
        .collect();

    for inv in organization.pending_invites(None, connection)? {
        members.push(DisplayOrganizationUser {
            user_id: inv.user_id,
            first_name: None,
            last_name: None,
            email: Some(inv.user_email),
            roles: inv.roles,
            invite_or_member: "invite".to_string(),
            invite_id: Some(inv.id),
        });
    }

    let payload = Payload::from_data(members, query_parameters.page(), query_parameters.limit(), None);
    Ok(WebPayload::new(StatusCode::OK, payload))
}

#[derive(Serialize, PartialEq, Debug)]
pub struct DisplayOrganizationUser {
    pub user_id: Option<Uuid>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub email: Option<String>,
    pub roles: Vec<Roles>,
    pub invite_or_member: String,
    pub invite_id: Option<Uuid>,
}

pub fn show_fee_schedule(
    (connection, parameters, user): (Connection, Path<PathParameters>, User),
) -> Result<HttpResponse, BigNeonError> {
    let connection = connection.get();
    let organization = Organization::find(parameters.id, connection)?;
    user.requires_scope_for_organization(Scopes::OrgWrite, &organization, connection)?;

    //This is an OrgOwner / Admin only call so we need to show the breakdown
    let fee_schedule = FeeSchedule::find(organization.fee_schedule_id, connection)?;
    let fee_schedule_ranges = fee_schedule.ranges(connection)?;

    Ok(HttpResponse::Ok().json(FeeScheduleWithRanges {
        id: fee_schedule.id,
        name: fee_schedule.name,
        version: fee_schedule.version,
        created_at: fee_schedule.created_at,
        ranges: fee_schedule_ranges,
    }))
}

pub fn add_fee_schedule(
    (connection, parameters, json, user): (Connection, Path<PathParameters>, Json<NewFeeScheduleRequest>, User),
) -> Result<HttpResponse, BigNeonError> {
    user.requires_scope(Scopes::OrgAdmin)?;
    let connection = connection.get();

    let new_fee_schedule = NewFeeSchedule {
        organization_id: parameters.id,
        name: json.name.clone(),
        ranges: json.into_inner().ranges,
    };
    let fee_schedule = new_fee_schedule.commit(Some(user.id()), connection)?;
    let fee_schedule_ranges = fee_schedule.ranges(connection)?;

    Organization::find(parameters.id, connection)?.add_fee_schedule(&fee_schedule, connection)?;

    Ok(HttpResponse::Created().json(FeeScheduleWithRanges {
        id: fee_schedule.id,
        name: fee_schedule.name,
        version: fee_schedule.version,
        created_at: fee_schedule.created_at,
        ranges: fee_schedule_ranges,
    }))
}

pub fn search_fans(
    (connection, path, query, user): (ReadonlyConnection, Path<PathParameters>, Query<PagingParameters>, User),
) -> Result<WebPayload<DisplayFan>, BigNeonError> {
    let connection = connection.get();
    let org = Organization::find(path.id, connection)?;
    user.requires_scope_for_organization(Scopes::OrgFans, &org, &connection)?;
    let payload = org.search_fans(
        query.get_tag("query"),
        query.page(),
        query.limit(),
        query
            .sort
            .as_ref()
            .map(|s| s.parse().unwrap_or(FanSortField::LastOrder))
            .unwrap_or(FanSortField::LastOrder),
        query.dir.unwrap_or(SortingDir::Desc),
        connection,
    )?;
    Ok(WebPayload::new(StatusCode::OK, payload))
}
