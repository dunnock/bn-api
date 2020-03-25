use std::collections::HashMap;
use std::sync::mpsc;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::thread;
use std::thread::JoinHandle;
use std::time::Duration;

use log::Level::*;

use crate::config::Config;
use crate::errors::*;
use crate::models::*;
use crate::utils::redis::*;
use actix::Addr;
use logging::*;
use uuid::Uuid;

pub struct RedisPubSubProcessor {
    config: Config,
    client: Option<redis::Client>,
    worker_threads: Vec<(Sender<()>, JoinHandle<Result<(), ApiError>>)>,
    websocket_clients: Arc<Mutex<HashMap<Uuid, Vec<Addr<EventWebSocket>>>>>,
}

impl RedisPubSubProcessor {
    pub fn new(
        config: Config,
        websocket_clients: Arc<Mutex<HashMap<Uuid, Vec<Addr<EventWebSocket>>>>>,
    ) -> RedisPubSubProcessor {
        let client = config
            .redis
            .clone()
            .map(|config| redis::Client::open(config.database_url).unwrap());
        RedisPubSubProcessor {
            config,
            client,
            websocket_clients,
            worker_threads: vec![],
        }
    }

    pub fn run_process(
        config: Config,
        mut conn: redis::Connection,
        websocket_clients: Arc<Mutex<HashMap<Uuid, Vec<Addr<EventWebSocket>>>>>,
        rx: Receiver<()>,
    ) -> Result<(), ApiError> {
        let mut pubsub = conn.as_pubsub();
        pubsub.set_read_timeout(config.redis.map(|config| config.read_timeout))?;

        // Todo: switch channels into enum
        pubsub.subscribe(RedisPubSubChannel::TicketRedemptions.to_string())?;

        loop {
            if rx.try_recv().is_ok() {
                jlog!(
                    Info,
                    "bigneon::redis_pubsub_processor",
                    "Stopping Redis PubSub processor",
                    {}
                );
                break;
            }

            match pubsub.get_message() {
                Ok(message) => match message.get_channel_name() {
                    "TicketRedemptions" => {
                        let payload: messages::TicketRedemption =
                            serde_json::from_str(&message.get_payload::<String>()?)?;
                        let clients = websocket_clients.clone();
                        let clients_mutex = clients.lock().unwrap();

                        if let Some(listeners) = clients_mutex.get(&payload.event_id) {
                            EventWebSocket::send_message(
                                &listeners,
                                EventWebSocketMessage::new(json!({
                                        "event_id": payload.event_id,
                                        "ticket_id": payload.ticket_id,
                                        "event_web_socket_type": EventWebSocketType::TicketRedemption
                                })),
                            );
                        }
                    }
                    _ => (),
                },
                Err(_) => {
                    thread::sleep(Duration::from_secs(1));
                }
            }
        }
        Ok(())
    }

    pub fn start(&mut self) {
        let (redis_pubsub_tx, redis_pubsub_rx) = mpsc::channel::<()>();
        let redis_pubsub_stop_signals = vec![redis_pubsub_tx.clone()];
        jlog!(
            Info,
            "bigneon::redis_pubsub_processor",
            "Redis PubSub processor starting",
            {}
        );

        let connection = self
            .client
            .as_ref()
            .expect("failed to start pubsub without redis connection")
            .get_connection()
            .expect("failed to start pubsub: redis connection failed");
        let config = self.config.clone();
        let websocket_clients = self.websocket_clients.clone();
        self.worker_threads.push((
            redis_pubsub_tx,
            thread::spawn(move || {
                let result = RedisPubSubProcessor::run_process(config, connection, websocket_clients, redis_pubsub_rx)
                    .map_err(|e| {
                        jlog!(
                            Error,
                            "bigneon::redis_pubsub_processor",
                            "Redis PubSub processor failed", {"error": e.to_string()}
                        );
                        e
                    });

                for signal in redis_pubsub_stop_signals {
                    match signal.send(()) {
                        Ok(_) => (),
                        Err(_) => (),
                    }
                }

                result
            }),
        ));
    }

    pub fn stop(&mut self) {
        for w in self.worker_threads.drain(..) {
            w.0.send(()).unwrap();
            w.1.join().unwrap().unwrap();
        }
    }
}
