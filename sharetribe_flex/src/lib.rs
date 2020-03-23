#![cfg_attr(not(debug_assertions), deny(unused_variables))]
#![cfg_attr(not(debug_assertions), deny(unused_imports))]
#![cfg_attr(not(debug_assertions), deny(dead_code))]
// Unused results is more often than not an error
#![deny(unused_must_use)]
#![cfg_attr(not(debug_assertions), deny(unused_extern_crates))]

pub const BASE_URI: &str = "https://flex-api.sharetribe.com/v1/";
// pub const BASE_URI: &str = "https://cc56e343.ngrok.io/";

mod auth;
mod error;
pub mod market_place_api;
mod response;
mod result;
mod util;

pub use error::ShareTribeError;
pub use market_place_api::marketplace_client::MarketplaceClient;
pub use response::*;
