use animo_db::models::{Listing, MarketplaceAccount, User};
use errors::AnimoError;

pub trait MarketplaceApi {
    fn link_user(&self, user: &User, account: &MarketplaceAccount) -> Result<String, AnimoError>;
    fn publish_listing(&self, listing: &Listing, account: &MarketplaceAccount) -> Result<String, AnimoError>;
}
