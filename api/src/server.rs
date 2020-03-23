use crate::config::Config;
use crate::database::*;
use crate::domain_events::DomainActionMonitor;
use crate::middleware::{ApiLogger, AppVersionHeader, DatabaseTransaction, Metatags};
use crate::models::*;
use crate::routing;
use crate::utils::redis::*;
use crate::utils::spotify;
use crate::utils::ServiceLocator;
use actix::Addr;
use actix_cors::Cors;
use actix_files as fs;
use actix_web::middleware::Logger;
use actix_web::{dev::ServiceRequest, http, HttpRequest, HttpResponse};
use actix_web::{web, web::Data, App, HttpServer};
use db::utils::errors::DatabaseError;
use log::Level::{Debug, Warn};
use std::collections::HashMap;
use std::error::Error;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

// Must be valid JSON
const LOGGER_FORMAT: &'static str = r#"{"level": "INFO", "target":"api::request", "remote_ip":"%a", "user_agent": "%{User-Agent}i", "request": "%r", "uri": "%U", "status_code": %s, "response_time": %D, "api_version":"%{x-app-version}o", "client_version": "%{X-API-Client-Version}i" }"#;

pub struct AppState {
    pub clients: Arc<Mutex<HashMap<Uuid, Vec<Addr<EventWebSocket>>>>>,
    pub config: Config,
    pub database: Database,
    pub database_ro: Database,
    pub service_locator: ServiceLocator,
}

impl AppState {
    pub fn new(
        config: Config,
        database: Database,
        database_ro: Database,
        clients: Arc<Mutex<HashMap<Uuid, Vec<Addr<EventWebSocket>>>>>,
    ) -> Result<AppState, DatabaseError> {
        Ok(AppState {
            database,
            database_ro,
            service_locator: ServiceLocator::new(&config)?,
            config,
            clients,
        })
    }
}

// actix:0.7 back compatibility
pub(crate) trait GetAppState {
    fn state(&self) -> Data<AppState>;
}
impl GetAppState for HttpRequest {
    fn state(&self) -> Data<AppState> {
        let data: &Data<AppState> = self.app_data().expect("critical: AppState not configured for App");
        data.clone()
    }
}
impl GetAppState for ServiceRequest {
    fn state(&self) -> Data<AppState> {
        let data: Data<AppState> = self.app_data().expect("critical: AppState not configured for App");
        data
    }
}

pub struct Server {
    pub config: Config,
}

impl Server {
    pub async fn start(
        config: Config,
        process_actions: bool,
        process_events: bool,
        process_http: bool,
        process_redis_pubsub: bool,
        process_actions_til_empty: bool,
    ) {
        jlog!(Debug, "api::server", "Server start requested", {"process_actions": process_actions, "process_events": process_events, "process_http":process_http, "process_actions_til_empty": process_actions_til_empty});
        let bind_addr = format!("{}:{}", config.api_host, config.api_port);

        let database = Database::from_config(&config).await;
        let database_ro = Database::readonly_from_config(&config).await;

        let mut domain_action_monitor = DomainActionMonitor::new(config.clone(), database.clone(), 1);
        if process_actions_til_empty {
            domain_action_monitor.run_til_empty().await.unwrap();
            return;
        }

        if process_actions || process_events {
            domain_action_monitor.start(process_actions, process_events);
        }

        if config.spotify_auth_token.is_some() {
            let token = config.spotify_auth_token.clone().unwrap();
            spotify::SINGLETON.set_auth_token(&token);
        }

        if process_http {
            info!("Listening on {}", bind_addr);

            let conf = config.clone();
            let static_file_conf = config.clone();

            let clients = Arc::new(Mutex::new(HashMap::new()));

            let mut redis_pubsub_processor = RedisPubSubProcessor::new(config.clone(), clients.clone());
            if process_redis_pubsub {
                redis_pubsub_processor.start();
            }

            //            let keep_alive = server::KeepAlive::Tcp(config.http_keep_alive);
            let mut server = HttpServer::new({
                move || {
                    App::new()
                        .data(
                            AppState::new(conf.clone(), database.clone(), database_ro.clone(), clients.clone())
                                .expect("Expected to generate app state"),
                        )
                        .wrap({
                            let mut cors_config = Cors::new();
                            cors_config = match conf.allowed_origins.as_ref() {
                                "*" => cors_config.send_wildcard(),
                                _ => cors_config.allowed_origin(&conf.allowed_origins),
                            };
                            cors_config
                                .allowed_methods(vec!["GET", "POST", "PUT", "PATCH", "DELETE"])
                                .allowed_headers(vec![
                                    http::header::AUTHORIZATION,
                                    http::header::ACCEPT,
                                    "X-API-Client-Version"
                                        .parse::<http::header::HeaderName>()
                                        .unwrap(),
                                ])
                                .allowed_header(http::header::CONTENT_TYPE)
                                .expose_headers(vec!["x-app-version", "x-cached-response"])
                                .max_age(3600)
                                .finish()
                        })
                        .wrap(Logger::new(LOGGER_FORMAT).exclude("/status"))
                        .wrap(ApiLogger::new())
                        .wrap(DatabaseTransaction::new())
                        .wrap(AppVersionHeader::new())
                        .wrap(Metatags::new(&conf))
                        .configure( routing::routes )
                        .configure( |conf| {
                            if let Some(static_file_path) = &static_file_conf.static_file_path {
                                conf.service(fs::Files::new("/", static_file_path));
                            }
                        })
                        .default_service(
                            web::get().to(|| HttpResponse::NotFound().json(json!({"error": "Not found"})))
                        )
                }
            })
                //            .keep_alive(keep_alive)
                .bind(&bind_addr)
                .unwrap_or_else(|_| panic!("Can not bind to {}", bind_addr));

            if let Some(workers) = config.actix.workers {
                server = server.workers(workers);
            }
            match server.run().await {
                Ok(_) => {}
                Err(e) => jlog!(Warn, "api::server", "Server exit with error", {"error": e.description()}),
            };

            if process_actions || process_events {
                domain_action_monitor.stop();
            }

            if process_redis_pubsub {
                redis_pubsub_processor.stop();
            }
        } else {
            domain_action_monitor.wait_for_end();
        }
    }
}
