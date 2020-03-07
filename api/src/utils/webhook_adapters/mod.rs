mod customer_io;
mod null_adapter;

use crate::errors::BigNeonError;
use serde_json::Value;
use std::collections::hash_map::RandomState;
use std::collections::HashMap;

pub use self::customer_io::*;
pub use self::null_adapter::*;

pub trait WebhookAdapter {
    fn initialize(&mut self, config: Value);
    fn send(&self, webhook_urls: &[String], payload: HashMap<String, Value, RandomState>) -> Result<(), BigNeonError>;
}
