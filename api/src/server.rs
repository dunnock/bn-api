use crate::config::Config;
use crate::db::*;
use crate::domain_events::DomainActionMonitor;
use crate::middleware::{AppVersionHeader, BigNeonLogger}; //, DatabaseTransaction, Metatags};
use crate::models::*;
use crate::routing;
use crate::utils::redis::*;
use crate::utils::spotify;
use crate::utils::ServiceLocator;
use actix::Addr;
use actix_web::{http, HttpRequest, dev::ServiceRequest};
use actix_web::middleware::Logger;
use actix_web::{fs::StaticFiles, server, App};
use actix_cors::Cors;
use bigneon_db::utils::errors::DatabaseError;
use log::Level::Debug;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

// Must be valid JSON
const LOGGER_FORMAT: &'static str = r#"{"level": "INFO", "target":"bigneon::request", "remote_ip":"%a", "user_agent": "%{User-Agent}i", "request": "%r", "status_code": %s, "response_time": %D, "api_version":"%{x-app-version}o", "client_version": "%{X-API-Client-Version}i" }"#;

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
    fn state(&self) -> &AppState;
}
impl GetAppState for HttpRequest {
    fn state(&self) -> &AppState {
        self.app_data().expect("critical: AppState not configured for App")
    }
}
impl GetAppState for ServiceRequest {
    fn state(&self) -> &AppState {
        self.app_data().expect("critical: AppState not configured for App")
    }
}

pub struct Server {
    pub config: Config,
}

impl Server {
    pub fn start(
        config: Config,
        process_actions: bool,
        process_events: bool,
        process_http: bool,
        process_redis_pubsub: bool,
        process_actions_til_empty: bool,
    ) {
        jlog!(Debug, "bigneon_api::server", "Server start requested", {"process_actions": process_actions, "process_events": process_events, "process_http":process_http, "process_actions_til_empty": process_actions_til_empty});
        let bind_addr = format!("{}:{}", config.api_host, config.api_port);

        let database = Database::from_config(&config);
        let database_ro = Database::readonly_from_config(&config);

        let mut domain_action_monitor = DomainActionMonitor::new(config.clone(), database.clone(), 1);
        if process_actions_til_empty {
            domain_action_monitor.run_til_empty().unwrap();
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

            let mut redis_pubsub_processor =
                RedisPubSubProcessor::new(config.clone(), database.clone(), clients.clone());
            if process_redis_pubsub {
                redis_pubsub_processor.start();
            }

            //            let keep_alive = server::KeepAlive::Tcp(config.http_keep_alive);
            let mut server = server::new({
                move || {
                    App::new()
                        .app_data(
                            AppState::new(conf.clone(), database.clone(), database_ro.clone(), clients.clone())
                                .expect("Expected to generate app state"),
                        )
                        .wrap(Logger::new(LOGGER_FORMAT))
                        .wrap_fn(BigNeonLogger::create())
                        .wrap_fn(DatabaseTransaction::create())
                        .wrap(AppVersionHeader::new())
                        /*.middleware(Metatags::new(
                            conf.ssr_trigger_header.clone(),
                            conf.ssr_trigger_value.clone(),
                            conf.front_end_url.clone(),
                            conf.app_name.clone(),
                        ))*/
                        .configure(|a| {
                            let mut cors_config = Cors::for_app(a);
                            match conf.allowed_origins.as_ref() {
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
                                .max_age(3600);

                            routing::routes(&mut cors_config)
                        })
                        .configure(|a| {
                            match &static_file_conf.static_file_path {
                                Some(static_file_path) => a.handler("/", StaticFiles::new(static_file_path).unwrap()),
                                None => a
                            }
                        })
                }
            })
                //            .keep_alive(keep_alive)
                .bind(&bind_addr)
                .unwrap_or_else(|_| panic!("Can not bind to {}", bind_addr));

            if let Some(workers) = config.actix.workers {
                server = server.workers(workers);
            }
            server.run();

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
