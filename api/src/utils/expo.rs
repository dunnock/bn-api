use crate::errors::*;
use crate::expo::*;
use log::Level::Debug;
use serde::export::Option;
use serde_json::Value;
use std::str::FromStr;
use tokio::prelude::*;

pub fn send_push_notification_async(
    tokens: &[String],
    body: &str,
    custom_data: Option<Value>,
) -> Box<dyn Future<Item = (), Error = BigNeonError>> {
    match send_push_notification(tokens, body, custom_data) {
        Ok(_) => Box::new(future::ok(())),
        Err(e) => Box::new(future::err(e)),
    }
}

pub fn send_push_notification(tokens: &[String], body: &str, custom_data: Option<Value>) -> Result<(), BigNeonError> {
    let push_notifier = PushNotifier::new().gzip_policy(GzipPolicy::Always);

    let mut msgs = vec![];
    for token in tokens {
        let push_token = PushToken::from_str(token).map_err(|e| ApplicationError::new(e))?;
        let mut msg = PushMessage::new(push_token).body(body);
        if custom_data != None {
            msg = msg.data(custom_data.clone().unwrap())
        }
        msgs.push(msg);
    }

    let result = push_notifier
        .send_push_notifications(&msgs)
        .map_err(|e| ApplicationError::new(e.to_string()))?;
    jlog!(Debug, &format!("Expo push notification response:{:?}", result));
    Ok(())
}
