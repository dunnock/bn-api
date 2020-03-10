use crate::errors::*;
use log::Level;
use twilio::OutboundMessage;

// TODO: it uses sync client under the hood, so will block executor
pub async fn send_sms_async(
    account_id: &str,
    api_key: &str,
    from: String,
    to: Vec<String>,
    body: &str,
) -> Result<(), BigNeonError> {
    let client = twilio::Client::new(account_id, api_key);
    for t in to.iter() {
        let message = OutboundMessage::new(&from, t, body);
        let message = match client.send_message(message) {
            Ok(m) => m,
            Err(e) => {
                jlog!(Level::Error, "Could not send to Twilio", { "error": e });
                return Err(e.into());
            }
        };

        jlog!(Level::Info, "Message sent via Twilio", {
            "to":message.to,
            "from": message.from,
            "body": message.body,
            "status": message.status
        });
    };

    Ok(())
}
