use animo_db::models::{Listing, MarketplaceAccount, User};
use errors::AnimoError;
use sharetribe_flex::market_place_api::endpoints::current_user::CreateCurrentUserRequest;
use sharetribe_flex::market_place_api::endpoints::own_listings::{CreateListingRequest, Price};
use sharetribe_flex::MarketplaceClient;
use utils::marketplace_api::MarketplaceApi;

pub struct SharetribeMarketplaceApi {
    client_id: String,
    // client_secret: String,
}

impl SharetribeMarketplaceApi {
    pub fn new(client_id: String, _client_secret: String) -> Self {
        Self {
            client_id,
            // client_secret,
        }
    }
}

impl MarketplaceApi for SharetribeMarketplaceApi {
    fn link_user(&self, user: &User, account: &MarketplaceAccount) -> Result<String, AnimoError> {
        let mut client = MarketplaceClient::with_anonymous_auth(self.client_id.clone());
        let user = CreateCurrentUserRequest {
            email: account.marketplace_user_id.clone(),
            password: account.marketplace_password.clone(),
            first_name: user.first_name.clone().unwrap_or("".to_string()),
            last_name: user.last_name.clone().unwrap_or("".to_string()),
            display_name: Some(user.full_name()),
            bio: None,
            public_data: None,
            protected_data: None,
            private_data: Some(json!({
            "animo_user_id": user.id
            })),
        };
        let account = client.current_user.create(user)?;
        Ok(account.id.to_string())
    }

    fn publish_listing(&self, listing: &Listing, account: &MarketplaceAccount) -> Result<String, AnimoError> {
        let mut client = MarketplaceClient::with_user_auth(
            self.client_id.clone(),
            account.marketplace_user_id.clone(),
            account.marketplace_password.clone(),
        );
        let listing_request = CreateListingRequest {
            title: listing.title.clone(),
            description: None,
            geolocation: None,
            price: Some(Price {
                amount: listing.asking_price_in_cents,
                currency: "USD".to_string(),
            }),
            public_data: None,
            private_data: Some(json!({
            "animo_listing_id": listing.id
            })),
            images: None,
        };
        Ok(client.own_listings.create(listing_request)?.id.to_string())
    }
}
