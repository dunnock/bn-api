use crate::errors::*;
use branch_rs::prelude::*;
use serde_json::Value;
use std::collections::HashMap;

pub trait DeepLinker {
    fn create_deep_link(&self, raw_link: &str) -> Result<String, BigNeonError>;
    fn create_deep_link_with_fallback(&self, raw_link: &str) -> String;
    fn create_deep_link_with_alias(&self, raw_link: &str, alias: &str) -> Result<String, BigNeonError>;
    fn create_with_custom_data(
        &self,
        fallback_link: &str,
        custom_data: HashMap<String, Value>,
    ) -> Result<String, BigNeonError>;
}

pub struct BranchDeepLinker {
    client: BranchClient,
}
impl BranchDeepLinker {
    pub fn new(url: String, branch_key: String, timeout: u64) -> BranchDeepLinker {
        BranchDeepLinker {
            client: BranchClient::new(url, branch_key, timeout),
        }
    }
}

impl DeepLinker for BranchDeepLinker {
    fn create_deep_link(&self, raw_link: &str) -> Result<String, BigNeonError> {
        Ok(self.client.links.create(DeepLink {
            data: DeepLinkData {
                desktop_url: Some(raw_link.to_string()),
                web_only: true,
                ios_url: Some(raw_link.to_string()),
                fallback_url: Some(raw_link.to_string()),
                android_url: Some(raw_link.to_string()),
                android_deeplink_path: Some("random".to_string()),
                ..Default::default()
            },
            ..Default::default()
        })?)
    }

    fn create_deep_link_with_fallback(&self, raw_link: &str) -> String {
        match self.create_deep_link(raw_link) {
            Ok(deep_link) => deep_link,
            Err(error) => {
                error!("BranchDeepLinker Error: {:?}", error);
                raw_link.to_string()
            }
        }
    }

    fn create_deep_link_with_alias(&self, raw_link: &str, alias: &str) -> Result<String, BigNeonError> {
        Ok(self.client.links.create(DeepLink {
            data: DeepLinkData {
                desktop_url: Some(raw_link.to_string()),
                web_only: true,
                ios_url: Some(raw_link.to_string()),
                fallback_url: Some(raw_link.to_string()),
                android_url: Some(raw_link.to_string()),
                android_deeplink_path: Some("random".to_string()),

                ..Default::default()
            },
            alias: Some(alias.to_string()),
            ..Default::default()
        })?)
    }

    fn create_with_custom_data(
        &self,
        fallback_link: &str,
        custom_data: HashMap<String, Value>,
    ) -> Result<String, BigNeonError> {
        Ok(self.client.links.create(DeepLink {
            data: DeepLinkData {
                desktop_url: Some(fallback_link.to_string()),
                custom_data,
                ..Default::default()
            },
            ..Default::default()
        })?)
    }
}
