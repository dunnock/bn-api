use crate::auth::user::User;
use crate::database::Connection;
use crate::errors::ApiError;
use crate::extractors::Json;
use crate::helpers::application;
use crate::models::PathParameters;
use crate::server::AppState;
use actix_web::{
    web::{Data, Path},
    HttpResponse,
};
use db::models::Listing;
use db::models::{Scopes, TicketInstance};
use uuid::Uuid;

pub fn create((user, conn, data): (User, Connection, Json<CreateListingRequest>)) -> Result<HttpResponse, ApiError> {
    let conn = conn.get();
    user.requires_scope(Scopes::ListingWrite)?;
    let data = data.into_inner();
    let listing = Listing::create(data.title.clone(), user.id(), data.asking_price_in_cents).commit(conn)?;
    let wallet = user.user.default_wallet(conn)?;
    for item in data.items {
        TicketInstance::add_to_listing(
            Some(user.id()),
            wallet.id,
            listing.id,
            item.ticket_type_id,
            item.quantity,
            conn,
        )?;
    }
    Ok(HttpResponse::Ok().json(json!({"id": listing.id})))
}

pub fn publish(
    (path, user, conn, state): (Path<PathParameters>, User, Connection, Data<AppState>),
) -> Result<HttpResponse, ApiError> {
    let conn = conn.get();
    user.requires_scope(Scopes::ListingWrite)?;
    let listing = Listing::find(path.id, conn)?;
    if listing.user_id != user.id() {
        return application::forbidden("You cannot publish this listing because you are not the owner");
    }
    let marketplace_account = user.user.marketplace_account(conn)?;
    let marketplace_account = if marketplace_account.is_none() {
        return application::unprocessable(
            "User does not have a marketplace account. First create a marketplace account and then try again",
        );
    } else {
        marketplace_account.unwrap()
    };

    // Send to market place
    let marketplace_api = state.service_locator.create_marketplace_api()?;
    let m_listing = marketplace_api.publish_listing(&listing, &marketplace_account)?;
    listing.set_published(m_listing, conn)?;
    unimplemented!()
}

#[derive(Deserialize)]
pub struct CreateListingRequest {
    pub title: String,
    pub items: Vec<AddListingItemRequest>,
    pub asking_price_in_cents: i64,
}

#[derive(Deserialize)]
pub struct AddListingItemRequest {
    pub ticket_type_id: Uuid,
    pub quantity: u32,
}
