// Websocket based on actix example https://github.com/actix/examples/blob/0.7/websocket/src/main.rs

use crate::models::*;
use actix::prelude::*;
use actix_web_actors::ws;
use std::time::{Duration, Instant};
use uuid::Uuid;

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);
const CLIENT_TIMEOUT: Duration = Duration::from_secs(30);

pub struct EventWebSocket {
    pub heartbeat: Instant,
    pub event_id: Uuid,
}

impl Actor for EventWebSocket {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, context: &mut Self::Context) {
        self.heartbeat(context);
    }
}

impl EventWebSocket {
    pub fn send_message(listeners: &[Addr<EventWebSocket>], message: EventWebSocketMessage) {
        for listener in listeners {
            if listener.connected() {
                if let Err(err) = listener.try_send(message.clone()) {
                    error!("Websocket send error: {:?}", err);
                }
            }
        }
    }

    pub fn new(event_id: Uuid) -> Self {
        Self {
            heartbeat: Instant::now(),
            event_id,
        }
    }

    fn heartbeat(&self, context: &mut <Self as Actor>::Context) {
        context.run_interval(HEARTBEAT_INTERVAL, |act, context| {
            context.ping("");
            if Instant::now().duration_since(act.heartbeat) > CLIENT_TIMEOUT {
                act.close(context);
            }
        });
    }

    pub fn close(&mut self, context: &mut <Self as Actor>::Context) {
        context.stop();

        let client_mutex = context.state().clients.clone();
        let mut clients = client_mutex.lock().unwrap();
        clients
            .entry(self.event_id)
            .and_modify(|listeners| listeners.retain(|l| l != &context.address()));
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for EventWebSocket {
    fn started(&mut self, context: &mut Self::Context) {
        let mut clients = context.state().clients.lock().unwrap();
        clients
            .entry(self.event_id)
            .or_insert(Vec::new())
            .push(context.address());
    }

    fn handle(&mut self, message: Result<ws::Message, ws::ProtocolError>, context: &mut Self::Context) {
        match message {
            Ok(ws::Message::Ping(message)) => {
                self.heartbeat = Instant::now();
                context.pong(&message);
            }
            Ok(ws::Message::Pong(_)) => {
                self.heartbeat = Instant::now();
            }
            Ok(ws::Message::Text(text)) => context.text(text),
            Ok(ws::Message::Binary(bin)) => context.binary(bin),
            Ok(ws::Message::Close(_)) => {
                self.close(context);
            },
            Err(_) => {
                self.close(context);
            }
        }
    }
}
