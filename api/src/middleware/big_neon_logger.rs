use actix_web::error;
use actix_web::http::header;
use actix_web::http::StatusCode;
use actix_web::middleware::Finished;
use actix_web::middleware::Logger;
use actix_web::middleware::Middleware;
use actix_web::middleware::Started;
use actix_web::FromRequest;
use actix_web::HttpRequest;
use actix_web::HttpResponse;
use crate::extractors::OptionalUser;
use log::Level;
use crate::server::AppState;

pub struct BigNeonLogger {
    logger: Logger,
}

impl BigNeonLogger {
    pub fn new(format: &str) -> BigNeonLogger {
        BigNeonLogger {
            logger: Logger::new(format),
        }
    }
}

impl Middleware<AppState> for BigNeonLogger {
    fn start(&self, req: &HttpRequest<AppState>) -> error::Result<Started> {
        self.logger.start(req)?;
        let user = OptionalUser::from_request(req, &());
        let ip_address = req.connection_info().remote().map(|i| i.to_string());
        let uri = req.uri().to_string();
        let method = req.method().to_string();
        let user_agent = if let Some(ua) = req.headers().get(header::USER_AGENT) {
            let s = ua.to_str().unwrap_or("");
            Some(s.to_string())
        } else {
            None
        };
        if uri != "/status" {
            jlog!(
                Level::Info,
                "bigneon_api::big_neon_logger",
                format!("{} {} starting", method, uri).as_str(),
                {
                    "user_id": user.ok().map(|u| u.0.map(|v| v.id())),
                    "ip_address": ip_address,
                    "uri": uri,
                    "method": method,
                    "user_agent": user_agent,
                    "api_version": env!("CARGO_PKG_VERSION")
            });
        }

        Ok(Started::Done)
    }

    fn finish(&self, req: &HttpRequest<AppState>, resp: &HttpResponse) -> Finished {
        match resp.error() {
            Some(error) => {
                let user = OptionalUser::from_request(req, &());
                let ip_address = req.connection_info().remote().map(|i| i.to_string());
                let uri = req.uri().to_string();
                let method = req.method().to_string();
                let level = if resp.status() == StatusCode::UNAUTHORIZED {
                    Level::Info
                } else if resp.status().is_client_error() {
                    Level::Warn
                } else {
                    Level::Error
                };
                let user_agent = if let Some(ua) = req.headers().get(header::USER_AGENT) {
                    let s = ua.to_str().unwrap_or("");
                    Some(s.to_string())
                } else {
                    None
                };

                jlog!(
                    level,
                    "bigneon_api::big_neon_logger",
                    &error.to_string(),
                    {
                        "user_id": user.ok().map(|u| u.0.map(|v| v.id())),
                        "ip_address": ip_address,
                        "uri": uri,
                        "method": method,
                        "api_version": env!("CARGO_PKG_VERSION"),
                        "user_agent": user_agent
                });

                Finished::Done
            }
            None => {
                if req.uri().to_string() == "/status" {
                    Finished::Done
                } else {
                    self.logger.finish(req, resp)
                }
            }
        }
    }
}
