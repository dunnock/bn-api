use crate::config::Config;
use crate::errors::*;
use crate::utils::webhook_adapters::{CustomerIoWebhookAdapter, NullAdapter, WebhookAdapter};
use db::prelude::*;
use diesel::PgConnection;
use serde_json;
use std::collections::HashMap;
use uuid::Uuid;

// TODO: it uses sync client under the hood, so will block executor
pub async fn send_webhook_async(
    tokens: &[String],
    body: &str,
    domain_event_publisher_id: Option<Uuid>,
    conn: &PgConnection,
    config: &Config,
) -> Result<(), ApiError> {
    send_webhook(tokens, body, domain_event_publisher_id, conn, config)
}

pub fn send_webhook(
    webhook_urls: &[String],
    body: &str,
    domain_event_publisher_id: Option<Uuid>,
    conn: &PgConnection,
    config: &Config,
) -> Result<(), ApiError> {
    let adapter = match domain_event_publisher_id {
        None => Box::new(NullAdapter::new()) as Box<dyn WebhookAdapter>,
        Some(id) => {
            let domain_event_publisher = DomainEventPublisher::find(id, conn)?;
            let mut adapter = match domain_event_publisher.adapter {
                None => Box::new(NullAdapter::new()) as Box<dyn WebhookAdapter>,
                Some(x) => match x {
                    WebhookAdapters::CustomerIo => {
                        Box::new(CustomerIoWebhookAdapter::new(config)) as Box<dyn WebhookAdapter>
                    }
                },
            };

            if let Some(config) = domain_event_publisher.adapter_config {
                adapter.initialize(config);
            };

            adapter
        }
    };

    let payload: HashMap<String, serde_json::Value> = serde_json::from_str(body)?;

    adapter.send(webhook_urls, payload)
}
