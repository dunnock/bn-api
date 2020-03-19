use crate::errors::*;
use crate::models::*;
use actix::prelude::*;
use serde_json::Value;

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub enum EventWebSocketType {
    TicketRedemption,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct EventWebSocketMessage {
    pub payload: Value,
}

impl EventWebSocketMessage {
    pub fn new(payload: Value) -> Self {
        Self { payload }
    }
}

impl Message for EventWebSocketMessage {
    type Result = Result<(), ApiError>;
}

impl Handler<EventWebSocketMessage> for EventWebSocket {
    type Result = Result<(), ApiError>;

    fn handle(&mut self, message: EventWebSocketMessage, context: &mut Self::Context) -> Self::Result {
        context.text(serde_json::to_string(&message.payload)?);
        Ok(())
    }
}
