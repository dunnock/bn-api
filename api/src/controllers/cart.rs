use actix_web::{HttpResponse, Path, State};
use crate::auth::user::User;
use bigneon_db::models::TicketType as Dbticket_types;
use bigneon_db::models::User as DbUser;
use bigneon_db::models::*;
use bigneon_db::utils::errors::Optional;
use bigneon_db::utils::rand::random_alpha_string;
use crate::config::Config;
use crate::db::Connection;
use diesel::pg::PgConnection;
use crate::errors::*;
use crate::extractors::*;
use crate::helpers::application;
use itertools::Itertools;
use log::Level::Debug;
use log::Level::Info;
use crate::models::*;
use crate::payments::AuthThenCompletePaymentBehavior;
use crate::payments::PaymentProcessor;
use crate::payments::PaymentProcessorBehavior;
use crate::payments::RedirectToPaymentPageBehavior;
use serde_json;
use serde_json::Value;
use crate::server::AppState;
use std::collections::HashMap;
use crate::utils::ServiceLocator;
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
pub struct CartItem {
    pub ticket_type_id: Uuid,
    pub quantity: u32,
    #[serde(default, deserialize_with = "deserialize_unless_blank")]
    pub redemption_code: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct UpdateCartRequest {
    pub items: Vec<CartItem>,
    pub box_office_pricing: Option<bool>,
    pub tracking_data: Option<Value>,
}

pub fn update_cart(
    (connection, json, user, request_info): (Connection, Json<UpdateCartRequest>, User, RequestInfo),
) -> Result<HttpResponse, BigNeonError> {
    let json = json.into_inner();
    jlog!(Debug, "Update Cart", {"cart": json, "user_id": user.id()});
    let connection = connection.get();

    let box_office_pricing = json.box_office_pricing.unwrap_or(false);
    if box_office_pricing {
        let mut ticket_type_ids = json.items.iter().map(|i| i.ticket_type_id).collect::<Vec<Uuid>>();
        ticket_type_ids.sort();
        ticket_type_ids.dedup();

        for organization in Organization::find_by_ticket_type_ids(ticket_type_ids, connection)? {
            user.requires_scope_for_organization(Scopes::BoxOfficeTicketRead, &organization, connection)?
        }
    }

    // Find the current cart of the user, if it exists.
    let mut cart = Order::find_or_create_cart(&user.user, connection)?;

    let order_items: Vec<UpdateOrderItem> = json
        .items
        .iter()
        .map(|i| UpdateOrderItem {
            quantity: i.quantity,
            ticket_type_id: i.ticket_type_id,
            redemption_code: i.redemption_code.clone(),
        })
        .collect();

    for order_item in &order_items {
        if !Dbticket_types::is_event_available_for_sale(&order_item.ticket_type_id, connection)? {
            return Ok(HttpResponse::BadRequest().json(json!({"error": "Event has not been published.".to_string()})));
        }
    }

    cart.update_quantities(user.id(), &order_items, box_office_pricing, false, connection)?;

    cart.set_browser_data(request_info.user_agent.clone(), false, connection)?;
    cart.set_tracking_data(json.tracking_data.clone(), Some(user.id()), connection)?;

    Ok(HttpResponse::Ok().json(Order::find(cart.id, connection)?.for_display(None, user.id(), connection)?))
}

pub fn duplicate(
    (connection, path, user): (Connection, Path<PathParameters>, User),
) -> Result<HttpResponse, BigNeonError> {
    let connection = connection.get();
    let order = Order::find(path.id, connection)?;
    let user_id = order.on_behalf_of_user_id.unwrap_or(order.user_id);

    if user_id != user.id() {
        return application::forbidden("This cart does not belong to you");
    }

    let duplicate_order = order.duplicate_order(connection)?;
    Ok(HttpResponse::Ok().json(duplicate_order.for_display(None, user.id(), connection)?))
}

pub fn destroy((connection, user): (Connection, User)) -> Result<HttpResponse, BigNeonError> {
    let connection = connection.get();

    // Find the current cart of the user, if it exists.
    let mut cart = Order::find_or_create_cart(&user.user, connection)?;
    cart.update_quantities(user.id(), &[], false, true, connection)?;

    Ok(HttpResponse::Ok().json(Order::find(cart.id, connection)?.for_display(None, user.id(), connection)?))
}

pub fn replace_cart(
    (connection, json, user, request_info): (Connection, Json<UpdateCartRequest>, User, RequestInfo),
) -> Result<HttpResponse, BigNeonError> {
    let json = json.into_inner();
    jlog!(Debug, "Replace Cart", {"cart": json, "user_id": user.id() });

    let connection = connection.get();

    let box_office_pricing = json.box_office_pricing.unwrap_or(false);
    if box_office_pricing {
        let mut ticket_type_ids = json.items.iter().map(|i| i.ticket_type_id).collect::<Vec<Uuid>>();
        ticket_type_ids.sort();
        ticket_type_ids.dedup();

        for organization in Organization::find_by_ticket_type_ids(ticket_type_ids, connection)? {
            user.requires_scope_for_organization(Scopes::BoxOfficeTicketRead, &organization, connection)?
        }
    }

    // Find the current cart of the user, if it exists.
    let mut cart = Order::find_or_create_cart(&user.user, connection)?;

    let order_items: Vec<UpdateOrderItem> = json
        .items
        .iter()
        .map(|i| UpdateOrderItem {
            quantity: i.quantity,
            ticket_type_id: i.ticket_type_id,
            redemption_code: i.redemption_code.clone(),
        })
        .collect();

    for order_item in &order_items {
        if !Dbticket_types::is_event_available_for_sale(&order_item.ticket_type_id, connection)? {
            return Ok(HttpResponse::BadRequest().json(json!({"error": "Event has not been published.".to_string()})));
        }
    }

    cart.update_quantities(user.id(), &order_items, box_office_pricing, true, connection)?;

    cart.set_browser_data(request_info.user_agent.clone(), false, connection)?;
    cart.set_tracking_data(json.tracking_data.clone(), Some(user.id()), connection)?;
    Ok(HttpResponse::Ok().json(Order::find(cart.id, connection)?.for_display(None, user.id(), connection)?))
}

pub fn show((connection, user): (Connection, User)) -> Result<HttpResponse, BigNeonError> {
    let connection = connection.get();
    let order = match Order::find_cart_for_user(user.id(), connection)? {
        Some(o) => o,
        None => return Ok(HttpResponse::Ok().json(json!({}))),
    };
    Ok(HttpResponse::Ok().json(order.for_display(None, user.id(), connection)?))
}

#[derive(Deserialize)]
pub struct CheckoutCartRequest {
    pub method: PaymentRequest,
    pub tracking_data: Option<serde_json::Value>,
}

#[derive(Deserialize)]
#[serde(tag = "type")]
pub enum PaymentRequest {
    External {
        #[serde(default, deserialize_with = "deserialize_unless_blank")]
        reference: Option<String>,
        external_payment_type: ExternalPaymentType,
        first_name: String,
        last_name: String,
        #[serde(default, deserialize_with = "deserialize_unless_blank")]
        email: Option<String>,
        #[serde(default, deserialize_with = "deserialize_unless_blank")]
        phone: Option<String>,
        #[serde(default, deserialize_with = "deserialize_unless_blank")]
        note: Option<String>,
    },
    Card {
        token: String,
        provider: PaymentProviders,
        save_payment_method: bool,
        set_default: bool,
    },
    Provider {
        provider: PaymentProviders,
    },
    PaymentMethod {
        #[serde(default, deserialize_with = "deserialize_unless_blank")]
        provider: Option<PaymentProviders>,
    },
    // Only for 0 amount carts
    Free,
}

pub fn clear_invalid_items((connection, user): (Connection, User)) -> Result<HttpResponse, BigNeonError> {
    let connection = connection.get();
    let mut order = match Order::find_cart_for_user(user.id(), connection)? {
        Some(o) => o,
        None => return application::unprocessable("No cart exists for user"),
    };

    if order.status != OrderStatus::Draft {
        return application::unprocessable("Could not complete this cart because it is not in the correct status");
    }
    info!("CART: Clearing invalid items");

    order.clear_invalid_items(user.id(), connection)?;
    Ok(HttpResponse::Ok().json(order.for_display(None, user.id(), connection)?))
}

pub fn checkout(
    (connection, json, user, state, request_info): (
        Connection,
        Json<CheckoutCartRequest>,
        User,
        State<AppState>,
        RequestInfo,
    ),
) -> Result<HttpResponse, BigNeonError> {
    // TODO: Change application::unprocesable's in this method to validation errors.
    let req = json.into_inner();

    info!("CART: Checking out");
    let mut order = match Order::find_cart_for_user(user.id(), connection.get())? {
        Some(o) => o,
        None => return application::unprocessable("No cart exists for user"),
    };
    order.lock_version(connection.get())?;

    if !order.items_valid_for_purchase(connection.get())? {
        return application::unprocessable("Could not complete this checkout because it contains invalid order items");
    }

    order.set_tracking_data(req.tracking_data.clone(), Some(user.id()), connection.get())?;

    let order_items = order.items(connection.get())?;

    //Assemble token ids and ticket instance ids for each asset in the order
    let mut tokens_per_asset: HashMap<Uuid, Vec<u64>> = HashMap::new();
    let mut wallet_id_per_asset: HashMap<Uuid, Uuid> = HashMap::new();
    for oi in &order_items {
        let tickets = TicketInstance::find_for_order_item(oi.id, connection.get())?;
        for ticket in &tickets {
            tokens_per_asset
                .entry(ticket.asset_id)
                .or_insert_with(|| Vec::new())
                .push(ticket.token_id as u64);
            wallet_id_per_asset.entry(ticket.asset_id).or_insert(ticket.wallet_id);
        }
    }
    info!("CART: Verifying asset");
    //Just confirming that the asset is setup correctly before proceeding to payment.
    for asset_id in tokens_per_asset.keys() {
        let asset = Asset::find(*asset_id, connection.get())?;
        if asset.blockchain_asset_id.is_none() {
            return application::internal_server_error(
                "Could not complete this checkout because the asset has not been assigned on the blockchain",
            );
        }
    }

    let payment_response = match &req.method {
        PaymentRequest::Free => {
            info!("CART: Received checkout for free cart");
            checkout_free(&connection, order, &user, &request_info)?
        }
        PaymentRequest::External {
            reference,
            external_payment_type,
            first_name,
            last_name,
            email,
            phone,
            note,
        } => {
            info!("CART: Received external payment");
            checkout_external(
                &connection,
                order,
                *external_payment_type,
                reference.clone(),
                first_name.to_string(),
                last_name.to_string(),
                email.clone(),
                phone.clone(),
                note.clone(),
                &user,
                &request_info,
            )?
        }
        PaymentRequest::PaymentMethod { provider } => {
            info!("CART: Received provider payment");
            let provider: PaymentProviders = match provider {
                Some(provider) => provider.clone(),
                None => match user.user.default_payment_method(connection.get()).optional()? {
                    Some(payment_method) => payment_method.name,
                    None => {
                        return application::unprocessable(
                            "Could not complete this cart because user has no default payment method",
                        );
                    }
                },
            };

            checkout_payment_processor(
                &connection,
                &mut order,
                None,
                &user,
                &state.config.primary_currency,
                provider.clone(),
                true,
                false,
                false,
                &state.service_locator,
                &state.config,
                &request_info,
            )?
        }
        PaymentRequest::Provider { provider } => checkout_payment_processor(
            &connection,
            &mut order,
            None,
            &user,
            &state.config.primary_currency,
            *provider,
            false,
            false,
            false,
            &state.service_locator,
            &state.config,
            &request_info,
        )?,
        PaymentRequest::Card {
            token,
            provider,
            save_payment_method,
            set_default,
        } => checkout_payment_processor(
            &connection,
            &mut order,
            Some(&token),
            &user,
            &state.config.primary_currency,
            *provider,
            false,
            *save_payment_method,
            *set_default,
            &state.service_locator,
            &state.config,
            &request_info,
        )?,
    };
    Ok(payment_response)
}

fn checkout_free(
    conn: &Connection,
    order: Order,
    user: &User,
    request_info: &RequestInfo,
) -> Result<HttpResponse, BigNeonError> {
    let conn = conn.get();
    if order.status != OrderStatus::Draft {
        return application::unprocessable("Could not complete this cart because it is not in the correct status");
    } else if order.calculate_total(conn)? > 0 {
        // TODO: make this line cleaner
        return application::unprocessable(
            "Could not use free payment method this cart because it has a total greater than zero",
        );
    }
    let mut order = order;
    order.add_free_payment(false, user.id(), conn)?;

    let mut order = Order::find(order.id, conn)?;
    order.set_browser_data(request_info.user_agent.clone(), true, conn)?;
    Ok(HttpResponse::Ok().json(json!(order.for_display(None, user.id(), conn)?)))
}

// TODO: This should actually probably move to an `orders` controller, since the
// user will not be calling this.
fn checkout_external(
    conn: &Connection,
    mut order: Order,
    external_payment_type: ExternalPaymentType,
    reference: Option<String>,
    first_name: String,
    last_name: String,
    email: Option<String>,
    phone: Option<String>,
    note: Option<String>,
    user: &User,
    request_info: &RequestInfo,
) -> Result<HttpResponse, BigNeonError> {
    let conn = conn.get();

    // User must have external checkout permissions for all events in the cart.
    for (event_id, _) in &order
        .items(conn)?
        .into_iter()
        .sorted_by_key(|oi| oi.event_id)
        .into_iter()
        .group_by(|oi| oi.event_id)
    {
        if let Some(event_id) = event_id {
            let organization = Organization::find_for_event(event_id, conn)?;
            user.requires_scope_for_organization(Scopes::OrderMakeExternalPayment, &organization, conn)?;
        }
    }

    if order.status != OrderStatus::Draft {
        return application::unprocessable("Could not complete this cart because it is not in the correct status");
    }

    let mut guest: Option<DbUser> = None;

    if let Some(ref e) = email {
        // Guest can be a deleted user
        guest = DbUser::find_by_email(e, true, conn).optional()?;
    };
    if guest.is_none() {
        if let Some(ref p) = phone {
            // Guest can be a deleted user
            guest = DbUser::find_by_phone(p, true, conn).optional()?;
        }
    }
    let guest = match guest {
        Some(g) => g,
        None => DbUser::create_stub(first_name, last_name, email, phone, Some(user.id()), conn)?,
    };

    if let Some(note) = note {
        order.create_note(note, user.id(), conn)?;
    }
    order.set_behalf_of_user(guest, user.id(), conn)?;
    let total = order.calculate_total(conn)?;

    if total == 0 {
        order.add_free_payment(true, user.id(), conn)?;
    } else {
        order.add_external_payment(reference, external_payment_type, user.id(), total, conn)?;
    }
    order.set_browser_data(request_info.user_agent.clone(), true, conn)?;

    let order = Order::find(order.id, conn)?;
    Ok(HttpResponse::Ok().json(json!(order.for_display(None, user.id(), conn)?)))
}

fn checkout_payment_processor(
    conn: &Connection,
    order: &mut Order,
    token: Option<&str>,
    auth_user: &User,
    currency: &str,
    provider: PaymentProviders,
    use_stored_payment: bool,
    save_payment_method: bool,
    set_default: bool,
    service_locator: &ServiceLocator,
    config: &Config,
    request_info: &RequestInfo,
) -> Result<HttpResponse, BigNeonError> {
    info!("CART: Executing provider payment");
    let connection = conn.get();

    if order.user_id != auth_user.id() {
        return application::forbidden("This cart does not belong to you");
    } else if order.status != OrderStatus::Draft {
        return application::unprocessable("Could not complete this cart because it is not in the correct status");
    } else if order.calculate_total(connection)? == 0 {
        return application::unprocessable("Could not complete this cart; only paid orders require payment processing");
    }

    // Can only have one event at a time because there are potentially different
    // payment gateway settings
    let mut events = order.events(connection)?;
    if events.len() != 1 {
        return application::unprocessable("Can't currently handle more than one event at the moment");
    };

    let event = events.remove(0);

    let client = service_locator.create_payment_processor(provider, &event.organization(connection)?)?;
    match client.behavior() {
        PaymentProcessorBehavior::RedirectToPaymentPage(behavior) => {
            return redirect_to_payment_page(&*behavior, &auth_user.user, order, conn.get(), config);
        }
        PaymentProcessorBehavior::AuthThenComplete(behavior) => {
            let token = if use_stored_payment {
                info!("CART: Using stored payment");
                match auth_user.user.payment_method(provider, connection).optional()? {
                    Some(payment_method) => payment_method.provider,
                    None => {
                        return application::unprocessable(
                            "Could not complete this cart because stored provider does not exist",
                        );
                    }
                }
            } else {
                info!("CART: Not using stored payment");
                let token = match token {
                    Some(t) => t,
                    None => {
                        return application::unprocessable("Could not complete this cart because no token provided");
                    }
                };

                if save_payment_method {
                    info!("CART: User has requested to save the payment method");
                    match auth_user.user.payment_method(provider, connection).optional()? {
                        Some(payment_method) => {
                            let behavior = match client.behavior() {
                                    PaymentProcessorBehavior::AuthThenComplete(b) => b,
                                    _ => return application::unprocessable(
                                        "Could not complete this cart using saved payment methods is not supported for this payment processor",
                                    )
                                };
                            let client_response =
                                behavior.update_repeat_token(&payment_method.provider, token, "Big Neon something")?;
                            let payment_method_parameters = PaymentMethodEditableAttributes {
                                provider_data: Some(client_response.to_json()?),
                            };
                            payment_method.update(&payment_method_parameters, auth_user.id(), connection)?;

                            payment_method.provider
                        }
                        None => {
                            let behavior = match client.behavior() {
                                    PaymentProcessorBehavior::AuthThenComplete(b) => b,
                                    _ => return application::unprocessable(
                                        "Could not complete this cart using saved payment methods is not supported for this payment processor",
                                    )
                                };
                            let repeat_token = behavior.create_token_for_repeat_charges(token, "Big Neon")?;
                            let _payment_method = PaymentMethod::create(
                                auth_user.id(),
                                provider,
                                set_default,
                                repeat_token.token.clone(),
                                repeat_token.to_json()?,
                            )
                            .commit(auth_user.id(), connection)?;
                            repeat_token.token
                        }
                    }
                } else {
                    token.to_string()
                }
            };

            return auth_then_complete(
                &*behavior,
                token,
                currency,
                order,
                auth_user,
                conn,
                &*client,
                request_info,
            );
        }
    };
}

fn auth_then_complete(
    client: &dyn AuthThenCompletePaymentBehavior,
    token: String,
    currency: &str,
    order: &mut Order,
    auth_user: &User,
    conn: &Connection,
    payment_processor: &dyn PaymentProcessor,
    request_info: &RequestInfo,
) -> Result<HttpResponse, BigNeonError> {
    let connection = conn.get();
    info!("CART: Auth'ing to payment provider");
    let amount = order.calculate_total(connection)?;
    let auth_result = client.auth(
        &token,
        amount,
        currency,
        "Big Neon Tickets",
        order.purchase_metadata(connection)?,
    )?;

    info!("CART: Saving payment to order");
    let payment = match order.add_credit_card_payment(
        auth_user.id(),
        amount,
        client.payment_provider(),
        auth_result.id.clone(),
        PaymentStatus::Authorized,
        auth_result.to_json()?,
        connection,
    ) {
        Ok(p) => p,
        Err(e) => {
            payment_processor.refund(&auth_result.id)?;
            return Err(e.into());
        }
    };

    conn.commit_transaction()?;
    conn.begin_transaction()?;

    info!("CART: Completing auth with payment provider");
    let charge_result = client.complete_authed_charge(&auth_result.id)?;
    info!("CART: Completing payment on order");
    info!("charge_result:{:?}", charge_result);
    match payment.mark_complete(charge_result.to_json()?, Some(auth_user.id()), connection) {
        Ok(_) => {
            let mut order = Order::find(order.id, connection)?;
            order.set_browser_data(request_info.user_agent.clone(), true, connection)?;
            Ok(HttpResponse::Ok().json(json!(order.for_display(None, auth_user.id(), connection)?)))
        }
        Err(e) => {
            payment_processor.refund(&auth_result.id)?;
            Err(e.into())
        }
    }
}

fn redirect_to_payment_page(
    client: &dyn RedirectToPaymentPageBehavior,
    user: &DbUser,
    order: &mut Order,
    conn: &PgConnection,
    config: &Config,
) -> Result<HttpResponse, BigNeonError> {
    if user.email.is_none() {
        return application::unprocessable("User must have an email to check out");
    }

    let amount = order.calculate_total(conn)?;

    let email = user.email.as_ref().unwrap().to_string();

    let ipn = Some(format!("{}/ipns/globee", config.api_base_url));

    let nonce = random_alpha_string(12);
    let response = client.create_payment_request(
        amount as f64 / 100_f64,
        email,
        order.id,
        ipn,
        Some(format!(
            "{}/payments/callback/{}/{}?success=true",
            &config.api_base_url, nonce, order.id,
        )),
        Some(format!(
            "{}/payments/callback/{}/{}?success=false",
            &config.api_base_url, nonce, order.id,
        )),
    )?;

    jlog!(Info, &format!("{} payment created", client.payment_provider()), {"order_id": order.id, "payment_provider_id": response.id});

    order.add_checkout_url(user.id, response.redirect_url.clone(), response.expires_at, conn)?;

    let external_reference = format!("globee-{}", response.id);

    order.add_provider_payment(
        Some(external_reference),
        client.payment_provider(),
        Some(user.id),
        amount,
        PaymentStatus::Requested,
        Some(nonce),
        json!(response.clone()),
        conn,
    )?;
    let order = Order::find(order.id, conn)?;
    Ok(HttpResponse::Ok().json(json!(order.for_display(None, user.id, conn)?)))
}
