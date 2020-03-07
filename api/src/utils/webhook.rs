use bigneon_db::prelude::*;
use crate::config::Config;
use diesel::PgConnection;
use crate::errors::*;
use serde_json;
use std::collections::HashMap;
use tokio::prelude::*;
use crate::utils::webhook_adapters::{CustomerIoWebhookAdapter, NullAdapter, WebhookAdapter};
use uuid::Uuid;

pub fn send_webhook_async(
    tokens: &[String],
    body: &str,
    domain_event_publisher_id: Option<Uuid>,
    conn: &PgConnection,
    config: &Config,
) -> Box<dyn Future<Item = (), Error = BigNeonError>> {
    match send_webhook(tokens, body, domain_event_publisher_id, conn, config) {
        Ok(_) => Box::new(future::ok(())),
        Err(e) => Box::new(future::err(e)),
    }
}

pub fn send_webhook(
    webhook_urls: &[String],
    body: &str,
    domain_event_publisher_id: Option<Uuid>,
    conn: &PgConnection,
    config: &Config,
) -> Result<(), BigNeonError> {
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
