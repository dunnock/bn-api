use crate::errors::ApiError;
use db::models::{Listing, MarketplaceAccount, User};

pub trait MarketplaceApi {
    fn link_user(&self, user: &User, account: &MarketplaceAccount) -> Result<String, ApiError>;
    fn publish_listing(&self, listing: &Listing, account: &MarketplaceAccount) -> Result<String, ApiError>;
}
