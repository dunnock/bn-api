use animo_db::prelude::*;
use actix_web::{HttpResponse};
use auth::user::User;
use errors::AnimoError;
use extractors::Json;
use db::Connection;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct CreateLootBoxRequest {
    pub name: String,
    pub promo_image_url: String,
    pub quantity: i64,
    pub price_in_cents: i64,
    pub contents: Vec<CreateLootBoxContentRequest>
}

#[derive(Deserialize)]
pub struct CreateLootBoxContentRequest {
    pub event_id: Uuid,
    pub min_rarity_id: Uuid,
    pub max_rarity_id: Uuid,
    pub quantity_per_box: i32
}

pub fn create((connection, data, user) : (Connection,  Json<CreateLootBoxRequest>, User)) -> Result<HttpResponse, AnimoError> {
    let conn = connection.get();
    let model = NewLootBox{
        name: data.name.clone(),
        promo_image_url: Some(data.promo_image_url.clone()),
        price_in_cents: data.price_in_cents
    };

    let loot_box  = model.commit(conn)?;

    let quantity = data.quantity;
    for content in data.into_inner().contents {


        let event = Event::find(content.event_id, conn)?;

        let org_wallet = Wallet::find_default_for_organization(event.organization_id, conn)?;

        user.requires_scope_for_organization_event(Scopes::LootBoxWrite, &event.organization(conn)?, &event, conn)?;
        let new_content = NewLootBoxContent{
            loot_box_id: loot_box.id,
            event_id: content.event_id,
            min_rarity_id: content.min_rarity_id,
            max_rarity_id: content.max_rarity_id,
            quantity_per_box: content.quantity_per_box
        };
        let c = new_content.commit(conn)?;

        LootBoxInstance::create_multiple(Some(user.id()), loot_box.id, quantity, &c, org_wallet.id, conn)?;
    }

    Ok(HttpResponse::Created().json(&loot_box))

}
