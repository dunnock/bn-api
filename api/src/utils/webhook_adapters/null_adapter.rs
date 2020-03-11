use crate::errors::{ApplicationError, BigNeonError};
use crate::utils::webhook_adapters::WebhookAdapter;
use log::Level::Debug;
use serde_json::Value;
use std::collections::hash_map::RandomState;
use std::collections::HashMap;

pub struct NullAdapter {}

impl NullAdapter {
    pub fn new() -> NullAdapter {
        NullAdapter {}
    }
}

impl WebhookAdapter for NullAdapter {
    fn initialize(&mut self, _config: Value) {}

    fn send(&self, webhook_urls: &[String], payload: HashMap<String, Value, RandomState>) -> Result<(), BigNeonError> {
        let client = reqwest::blocking::Client::new();
        for webhook_url in webhook_urls {
            let mut resp = client
                .post(webhook_url)
                .json(&payload)
                .send()
                .map_err(|_err| ApplicationError::new("Error making webhook request".to_string()))?;

            let text = resp
                .text()
                .map_err(|_err| ApplicationError::new("Error making webhook request".to_string()))?;

            jlog!(Debug, "bigneon::domain_actions", "Response from customer.io", {"text": text, "status": resp.status().to_string()});
            resp.error_for_status()?;
        }
        Ok(())
    }
}
