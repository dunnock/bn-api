use crate::auth::auth_client::AuthClient;
use crate::auth::token::{GrantType, TokenRequest};
use crate::market_place_api::endpoints::current_user::CurrentUserEndpoint;
use crate::market_place_api::endpoints::own_listings::OwnListingEndpoint;
use std::sync::{Arc, RwLock};

pub struct MarketplaceClient {
    pub current_user: CurrentUserEndpoint,
    pub own_listings: OwnListingEndpoint,
}

impl MarketplaceClient {
    pub fn new(auth: AuthClient) -> MarketplaceClient {
        let auth = Arc::new(RwLock::new(auth));
        MarketplaceClient {
            current_user: CurrentUserEndpoint::new(auth.clone()),
            own_listings: OwnListingEndpoint::new(auth),
        }
    }

    pub fn with_anonymous_auth(client_id: String) -> MarketplaceClient {
        let credentials = TokenRequest {
            client_id,
            scope: "public-read".to_string(),
            grant_type: GrantType::Anonymous,
        };
        let auth = AuthClient::new(credentials);
        MarketplaceClient::new(auth)
    }
    pub fn with_user_auth(client_id: String, username: String, password: String) -> MarketplaceClient {
        let credentials = TokenRequest {
            client_id,
            scope: "user".to_string(),
            grant_type: GrantType::Password { username, password },
        };
        let auth = AuthClient::new(credentials);
        MarketplaceClient::new(auth)
    }
}
