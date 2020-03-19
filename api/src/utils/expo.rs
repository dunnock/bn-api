use crate::errors::*;
use crate::expo::*;
use log::Level::Debug;
use serde::export::Option;
use serde_json::Value;
use std::str::FromStr;

// TODO: it is actually sync under the hood and will block executor
pub async fn send_push_notification_async(
    tokens: &[String],
    body: &str,
    custom_data: Option<Value>,
) -> Result<(), ApiError> {
    send_push_notification(tokens, body, custom_data)
}

pub fn send_push_notification(tokens: &[String], body: &str, custom_data: Option<Value>) -> Result<(), ApiError> {
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
