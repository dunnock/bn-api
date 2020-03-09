use crate::auth::user::User;
use crate::db::Connection;
use crate::errors::*;
use crate::extractors::*;
use crate::helpers::application;
use crate::models::{AdminDisplayTicketType, EventTicketPathParameters, PathParameters};
use crate::server::AppState;
use crate::utils::serializers::default_as_true;
use actix_web::{HttpResponse, web::{Path, Query, Data}};
use bigneon_db::dev::times;
use bigneon_db::models::*;
use chrono::prelude::*;
use diesel::PgConnection;
use log::Level::Debug;
use serde_with::rust::double_option;
use tari_client::MessagePayloadCreateAsset as TariNewAsset;
use uuid::Uuid;

#[derive(Clone, Deserialize)]
pub struct CreateTicketPricingRequest {
    pub name: String,
    pub price_in_cents: i64,
    pub start_date: NaiveDateTime,
    pub end_date: NaiveDateTime,
    pub is_box_office_only: Option<bool>,
}

#[derive(Clone, Deserialize)]
pub struct CreateMultipleTicketTypeRequest {
    pub ticket_types: Vec<CreateTicketTypeRequest>,
}

#[derive(Clone, Deserialize)]
pub struct CreateTicketTypeRequest {
    pub name: String,
    #[serde(default, deserialize_with = "deserialize_unless_blank")]
    pub description: Option<String>,
    pub capacity: u32,
    pub start_date: Option<NaiveDateTime>,
    pub parent_id: Option<Uuid>,
    pub end_date: Option<NaiveDateTime>,
    pub end_date_type: Option<TicketTypeEndDateType>,
    #[serde(default)]
    pub ticket_pricing: Vec<CreateTicketPricingRequest>,
    pub increment: Option<i32>,
    pub limit_per_person: i32,
    pub price_in_cents: i64,
    pub visibility: TicketTypeVisibility,
    #[serde(default)]
    pub additional_fee_in_cents: Option<i64>,
    #[serde(default)]
    pub rank: i64,
    #[serde(default = "default_as_true")]
    pub web_sales_enabled: bool,
    #[serde(default = "default_as_true")]
    pub box_office_sales_enabled: bool,
    #[serde(default = "default_as_true")]
    pub app_sales_enabled: bool,
}

impl Default for CreateTicketTypeRequest {
    fn default() -> Self {
        CreateTicketTypeRequest {
            name: "".to_string(),
            description: None,
            capacity: 0,
            start_date: None,
            parent_id: None,
            end_date: None,
            end_date_type: Some(TicketTypeEndDateType::Manual),
            ticket_pricing: vec![],
            increment: None,
            limit_per_person: 0,
            price_in_cents: 0,
            visibility: TicketTypeVisibility::Always,
            additional_fee_in_cents: None,
            rank: 0,
            web_sales_enabled: true,
            box_office_sales_enabled: true,
            app_sales_enabled: true,
        }
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct UpdateTicketPricingRequest {
    pub id: Option<Uuid>,
    pub name: Option<String>,
    pub start_date: Option<NaiveDateTime>,
    pub end_date: Option<NaiveDateTime>,
    pub price_in_cents: Option<i64>,
    pub is_box_office_only: Option<bool>,
}

#[derive(Deserialize, Serialize, Debug, Default)]
pub struct UpdateTicketTypeRequest {
    pub name: Option<String>,
    #[serde(default, deserialize_with = "double_option_deserialize_unless_blank")]
    pub description: Option<Option<String>>,
    pub capacity: Option<u32>,
    #[serde(deserialize_with = "double_option::deserialize")]
    pub start_date: Option<Option<NaiveDateTime>>,
    pub end_date: Option<Option<NaiveDateTime>>,
    pub end_date_type: Option<TicketTypeEndDateType>,
    pub ticket_pricing: Option<Vec<UpdateTicketPricingRequest>>,
    pub increment: Option<i32>,
    pub limit_per_person: Option<i32>,
    pub price_in_cents: Option<i64>,
    #[serde(default)]
    pub visibility: Option<TicketTypeVisibility>,
    #[serde(deserialize_with = "double_option::deserialize")]
    pub parent_id: Option<Option<Uuid>>,
    #[serde(default)]
    pub additional_fee_in_cents: Option<i64>,
    #[serde(default)]
    pub web_sales_enabled: Option<bool>,
    #[serde(default)]
    pub box_office_sales_enabled: Option<bool>,
    #[serde(default)]
    pub app_sales_enabled: Option<bool>,
    pub rank: Option<i32>,
}

#[derive(Serialize, Deserialize)]
pub struct DisplayCreatedTicket {
    pub id: Uuid,
}

pub fn create(
    (connection, path, data, user, state): (
        Connection,
        Path<PathParameters>,
        Json<CreateTicketTypeRequest>,
        User,
        Data<AppState>,
    ),
) -> Result<HttpResponse, BigNeonError> {
    let connection = connection.get();
    let event = Event::find(path.id, connection)?;
    let organization = event.organization(connection)?;
    user.requires_scope_for_organization_event(Scopes::TicketTypeWrite, &organization, &event, connection)?;

    let created_ticket_types = create_ticket_types(
        &event,
        &organization,
        &user,
        vec![data.into_inner()],
        &state,
        connection,
    )?;
    Ok(HttpResponse::Created().json(&created_ticket_types[0]))
}

pub fn create_multiple(
    (connection, path, data, user, state): (
        Connection,
        Path<PathParameters>,
        Json<CreateMultipleTicketTypeRequest>,
        User,
        Data<AppState>,
    ),
) -> Result<HttpResponse, BigNeonError> {
    let connection = connection.get();
    let data = data.into_inner();
    let event = Event::find(path.id, connection)?;
    let organization = event.organization(connection)?;
    user.requires_scope_for_organization_event(Scopes::TicketTypeWrite, &organization, &event, connection)?;

    let created_ticket_types =
        create_ticket_types(&event, &organization, &user, data.ticket_types, &state, connection)?;
    Ok(HttpResponse::Created().json(created_ticket_types))
}

#[derive(Deserialize, Serialize)]
pub struct TicketTypesResponse {
    pub ticket_types: Vec<AdminDisplayTicketType>,
}

pub fn index(
    (connection, path, query_parameters, user): (Connection, Path<PathParameters>, Query<PagingParameters>, User),
) -> Result<HttpResponse, BigNeonError> {
    let connection = connection.get();
    let event = Event::find(path.id, connection)?;
    let organization = event.organization(connection)?;
    user.requires_scope_for_organization_event(Scopes::TicketTypeRead, &organization, &event, connection)?;

    let fee_schedule = FeeSchedule::find(organization.fee_schedule_id, connection)?;
    //TODO refactor using paging params
    let ticket_types = TicketType::find_by_event_id(path.id, false, None, connection)?;
    let mut payload = Payload::new(vec![], query_parameters.into_inner().into());

    for t in ticket_types {
        payload
            .data
            .push(AdminDisplayTicketType::from_ticket_type(&t, &fee_schedule, connection)?);
    }
    payload.paging.limit = payload.data.len() as u32;
    payload.paging.total = payload.data.len() as u64;

    Ok(HttpResponse::Ok().json(&payload))
}

pub fn cancel(
    (connection, path, user, state): (Connection, Path<EventTicketPathParameters>, User, Data<AppState>),
) -> Result<HttpResponse, BigNeonError> {
    let connection = connection.get();
    let event = Event::find(path.event_id, connection)?;
    let organization = event.organization(connection)?;
    user.requires_scope_for_organization_event(Scopes::TicketTypeWrite, &organization, &event, connection)?;

    let ticket_type = TicketType::find(path.ticket_type_id, connection)?;
    let ticket_type = ticket_type.cancel(connection)?;

    // Reduce holds to quantity sold
    for hold in Hold::find_by_ticket_type(ticket_type.id, connection)?
        .into_iter()
        .filter(|h| h.parent_hold_id.is_none())
    {
        hold.remove_available_quantity(Some(user.id()), connection)?;
        if hold.quantity(connection)?.0 == 0 {
            hold.destroy(Some(user.id()), connection)?;
        }
    }

    let valid_unsold_ticket_count = ticket_type.valid_unsold_ticket_count(connection)?;
    nullify_tickets(
        state,
        organization,
        &ticket_type,
        valid_unsold_ticket_count,
        user.id(),
        connection,
    )?;

    // if there are no sold tickets, delete the ticket type
    if ticket_type.valid_sold_and_reserved_ticket_count(connection)? == 0 {
        ticket_type.delete(connection)?;
    }

    Ok(HttpResponse::Ok().finish())
}

pub fn update(
    (connection, path, data, user, state): (
        Connection,
        Path<EventTicketPathParameters>,
        Json<UpdateTicketTypeRequest>,
        User,
        Data<AppState>,
    ),
) -> Result<HttpResponse, BigNeonError> {
    let connection = connection.get();
    let event = Event::find(path.event_id, connection)?;
    let organization = event.organization(connection)?;
    let fee_schedule_id = organization.fee_schedule_id;
    user.requires_scope_for_organization_event(Scopes::TicketTypeWrite, &organization, &event, connection)?;

    let data = data.into_inner();
    jlog!(Debug, "Updating ticket type", {"ticket_type_id": path.ticket_type_id, "event_id":event.id, "request": &data});
    let ticket_type = TicketType::find(path.ticket_type_id, connection)?;
    if let Some(requested_capacity) = data.capacity {
        //Check that requested ticket capacity is less than max_tickets_per_ticket_type
        if requested_capacity as i64 > organization.max_instances_per_ticket_type {
            return application::unprocessable(
                "Requested capacity larger than organization maximum tickets per ticket type",
            );
        }

        let valid_ticket_count = ticket_type.valid_ticket_count(connection)?;

        if valid_ticket_count < requested_capacity {
            jlog!(Debug, "Update ticket type: Capacity increased", {"ticket_type_id": path.ticket_type_id, "new_capacity": requested_capacity, "old_capacity": valid_ticket_count});
            let starting_tari_id = ticket_type.ticket_count(connection)?;
            let additional_ticket_count = requested_capacity - valid_ticket_count;
            let asset = Asset::find_by_ticket_type(ticket_type.id, connection)?;
            let org_wallet = Wallet::find_default_for_organization(event.organization_id, connection)?;
            //Issue more tickets locally
            TicketInstance::create_multiple(
                asset.id,
                starting_tari_id,
                additional_ticket_count,
                org_wallet.id,
                connection,
            )?;
            //Issue more tickets on chain
            match asset.blockchain_asset_id {
                Some(a) => state.config.tari_client.modify_asset_increase_supply(
                    &org_wallet.secret_key,
                    &org_wallet.public_key,
                    &a,
                    requested_capacity as u64,
                )?,
                None => return application::internal_server_error(
                    "Could not complete capacity increase because the asset has not been assigned on the blockchain",
                ),
            }
        } else if valid_ticket_count > requested_capacity {
            jlog!(Debug, "Update ticket type: Capacity decreased", {"ticket_type_id": path.ticket_type_id, "new_capacity": requested_capacity, "old_capacity": valid_ticket_count});
            let nullify_ticket_count = valid_ticket_count - requested_capacity;
            nullify_tickets(
                state,
                organization,
                &ticket_type,
                nullify_ticket_count,
                user.id(),
                connection,
            )?;
        }
    }

    //Update the editable attributes of the ticket type
    let update_parameters = TicketTypeEditableAttributes {
        name: data.name.clone(),
        description: data.description.clone(),
        start_date: data.start_date,
        end_date: data.end_date,
        end_date_type: data.end_date_type,
        web_sales_enabled: data.web_sales_enabled,
        box_office_sales_enabled: data.box_office_sales_enabled,
        increment: data.increment,
        limit_per_person: data.limit_per_person,
        price_in_cents: data.price_in_cents,
        visibility: data.visibility,
        parent_id: data.parent_id,
        additional_fee_in_cents: data.additional_fee_in_cents,
        app_sales_enabled: data.app_sales_enabled,
        rank: data.rank,
    };
    let updated_ticket_type = ticket_type.update(update_parameters, Some(user.id()), connection)?;

    if let Some(ref data_ticket_pricing) = data.ticket_pricing {
        //Retrieve the current list of pricing associated with this ticket_type and remove unwanted pricing
        let ticket_pricing = updated_ticket_type.ticket_pricing(false, connection)?;
        for current_ticket_pricing in &ticket_pricing {
            let mut found_flag = false;
            for request_ticket_pricing in data_ticket_pricing {
                if request_ticket_pricing.id.is_some()
                    && current_ticket_pricing.id == request_ticket_pricing.id.unwrap()
                {
                    found_flag = true;
                    break;
                }
            }
            if !found_flag {
                current_ticket_pricing.destroy(Some(user.id()), connection)?;
            }
        }

        //Update the editable attributes for remaining ticket pricing
        for current_ticket_pricing in data_ticket_pricing {
            if let Some(current_ticket_pricing_id) = current_ticket_pricing.id {
                //Update the ticket pricing
                let update_parameters = TicketPricingEditableAttributes {
                    name: current_ticket_pricing.name.clone(),
                    price_in_cents: current_ticket_pricing.price_in_cents,
                    start_date: current_ticket_pricing.start_date,
                    end_date: current_ticket_pricing.end_date,
                    is_box_office_only: current_ticket_pricing.is_box_office_only,
                };
                let found_index = ticket_pricing
                    .iter()
                    .position(|ref r| r.id == current_ticket_pricing_id);
                match found_index {
                    Some(index) => ticket_pricing[index].update(update_parameters, Some(user.id()), connection)?,
                    None => {
                        return application::internal_server_error(&format!(
                            "Unable to find specified ticket pricing with id {}",
                            current_ticket_pricing_id
                        ));
                    }
                };
            } else if let (Some(name), Some(price_in_cents), Some(start_date), Some(end_date)) = (
                current_ticket_pricing.name.clone(),
                current_ticket_pricing.price_in_cents,
                current_ticket_pricing.start_date,
                current_ticket_pricing.end_date,
            ) {
                //Only create a new pricing entry if all of its required data was provided
                //Add new ticket pricing
                let _pricing_result = updated_ticket_type.add_ticket_pricing(
                    name,
                    start_date,
                    end_date,
                    price_in_cents,
                    current_ticket_pricing.is_box_office_only.unwrap_or(false),
                    None,
                    Some(user.id()),
                    connection,
                )?;
            } else {
                //TODO send error when all data was not specified
            }
        }
        updated_ticket_type.validate_ticket_pricing(connection)?;
    }

    let result = AdminDisplayTicketType::from_ticket_type(
        &(TicketType::find(path.ticket_type_id, connection)?),
        &FeeSchedule::find(fee_schedule_id, connection)?,
        connection,
    )?;

    Ok(HttpResponse::Ok().json(result))
}

fn nullify_tickets(
    state: Data<AppState>,
    organization: Organization,
    ticket_type: &TicketType,
    quantity: u32,
    user_id: Uuid,
    connection: &PgConnection,
) -> Result<(), BigNeonError> {
    let asset = Asset::find_by_ticket_type(ticket_type.id, connection)?;
    let org_wallet = Wallet::find_default_for_organization(organization.id, connection)?;
    //Nullify tickets locally
    let tickets = TicketInstance::nullify_tickets(asset.id, quantity, user_id, connection)?;
    //Nullify tickets on chain
    if tickets.len() == quantity as usize {
        let tari_ids: Vec<u64> = (0..tickets.len())
            .map(|i| tickets[i as usize].token_id as u64)
            .collect();
        match asset.blockchain_asset_id {
            Some(a) => {
                state.config.tari_client.modify_asset_nullify_tokens(
                    &org_wallet.secret_key,
                    &org_wallet.public_key,
                    &a,
                    tari_ids,
                )?;
            }
            None => {
                application::internal_server_error::<HttpResponse>(
                    "Could not complete capacity increase because the asset has not been assigned on the blockchain",
                )?;
            }
        }
    } else {
        application::internal_server_error::<HttpResponse>(&format!(
            "Unable to nullify the requested number ({}) of ticket instances",
            quantity
        ))?;
    }

    Ok(())
}

pub(crate) fn create_ticket_type_blockchain_assets(
    event: &Event,
    ticket_types: &[TicketType],
    state: &Data<AppState>,
    connection: &PgConnection,
) -> Result<(), BigNeonError> {
    //Retrieve default wallet
    let org_wallet = Wallet::find_default_for_organization(event.organization_id, connection)?;

    // Only create the blockchain assets after all of the ticket types have succeeded
    for data in ticket_types {
        let tari_asset_id = state.config.tari_client.create_asset(
            &org_wallet.secret_key,
            &org_wallet.public_key,
            TariNewAsset {
                name: format!("{}.{}", event.id, data.name),
                total_supply: data.valid_ticket_count(connection)? as u64,
                authorised_signers: Vec::new(),
                rule_flags: 0,
                rule_metadata: "".to_string(),
                expiry_date: data.end_date(connection)?.timestamp(),
            },
        )?;
        let asset = Asset::find_by_ticket_type(data.id, connection)?;
        asset.update_blockchain_id(tari_asset_id, connection)?;
    }

    Ok(())
}

fn create_ticket_types(
    event: &Event,
    organization: &Organization,
    user: &User,
    data: Vec<CreateTicketTypeRequest>,
    state: &Data<AppState>,
    connection: &PgConnection,
) -> Result<Vec<DisplayCreatedTicket>, BigNeonError> {
    //Check that any requested ticket capacity is less than max_instances_per_ticket_type
    if data
        .iter()
        .find(|tt| tt.capacity as i64 > organization.max_instances_per_ticket_type)
        .is_some()
    {
        application::internal_server_error::<HttpResponse>(
            "Requested capacity larger than organization maximum tickets per ticket type",
        )?;
    }

    //Add new ticket types
    let mut results = Vec::<TicketType>::new();
    let org_wallet = Wallet::find_default_for_organization(event.organization_id, connection)?;

    for ticket_type_data in data.iter() {
        let ticket_type = event.add_ticket_type(
            ticket_type_data.name.clone(),
            ticket_type_data.description.clone(),
            ticket_type_data.capacity,
            ticket_type_data.start_date.or(if ticket_type_data.parent_id.is_some() {
                None
            } else {
                Some(times::zero())
            }),
            ticket_type_data.end_date,
            ticket_type_data.end_date_type.unwrap_or(TicketTypeEndDateType::Manual),
            Some(org_wallet.id),
            ticket_type_data.increment,
            ticket_type_data.limit_per_person,
            ticket_type_data.price_in_cents,
            ticket_type_data.visibility,
            ticket_type_data.parent_id,
            ticket_type_data.additional_fee_in_cents.unwrap_or(0),
            ticket_type_data.app_sales_enabled,
            ticket_type_data.web_sales_enabled,
            ticket_type_data.box_office_sales_enabled,
            Some(user.id()),
            connection,
        )?;
        //Add each ticket pricing entry for newly created ticket type
        for current_pricing_entry in &ticket_type_data.ticket_pricing {
            let _pricing_result = ticket_type.add_ticket_pricing(
                current_pricing_entry.name.clone(),
                current_pricing_entry.start_date,
                current_pricing_entry.end_date,
                current_pricing_entry.price_in_cents,
                current_pricing_entry.is_box_office_only.unwrap_or(false),
                None,
                Some(user.id()),
                connection,
            )?;
        }

        ticket_type.validate_ticket_pricing(connection)?;
        results.push(ticket_type);
    }

    create_ticket_type_blockchain_assets(event, &results, state, connection)?;

    Ok(results.iter().map(|r| DisplayCreatedTicket { id: r.id }).collect())
}
