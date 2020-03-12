use crate::extractors::Uuid;
use actix_web::error;
use actix_web::http::header;
use actix_web::http::StatusCode;
use actix_web::dev::{ServiceRequest, ServiceResponse, MessageBody};
use log::Level;

pub struct BigNeonLogger;

impl BigNeonLogger {
    // log message at the start of request lifecycle
    pub fn start(sreq: &ServiceRequest) -> RequestLogData {
        let data = RequestLogData::from(sreq);
        if data.uri != "/status" {
            jlog!(
                Level::Info,
                "bigneon_api::big_neon_logger",
                format!("{} {} starting", data.method, data.uri).as_str(),
                {
                    "user_id": data.user,
                    "ip_address": data.ip_address,
                    "uri": data.uri,
                    "method": data.method,
                    "user_agent": data.user_agent,
                    "api_version": env!("CARGO_PKG_VERSION")
            });
        };
        data
    }

    // log message at the end of request lifecycle
    pub fn finish<B: MessageBody>(data: &RequestLogData, resp: error::Result<ServiceResponse<B>>) -> error::Result<ServiceResponse<B>> {
        let error = match resp {
            Err(ref error) => Some(error),
            Ok(ref resp) => resp.response().error(),
        };
        if let Some(error) = error {
            let level = match error.as_response_error().status_code() {
                StatusCode::UNAUTHORIZED => Level::Info,
                s if s.is_client_error() => Level::Warn,
                _ => Level::Error,
            };
            jlog!(
                level,
                "bigneon_api::big_neon_logger",
                &error.to_string(),
                {
                    "user_id": data.user,
                    "ip_address": data.ip_address,
                    "uri": data.uri,
                    "method": data.method,
                    "api_version": env!("CARGO_PKG_VERSION"),
                    "user_agent": data.user_agent
            });
        };
        resp
    }
}

pub struct RequestLogData {
    user: Option<uuid::Uuid>, // NOTE: this used to be Option<Option<uuid::Uuid>>
    ip_address: Option<String>,
    method: String,
    user_agent: Option<String>,
    uri: String,
}
impl RequestLogData {
    fn from(req: &ServiceRequest) -> Self {
        let uri = req.uri().to_string();
        let user = Uuid::from_request(req).ok();
        let ip_address = req.connection_info().remote().map(|i| i.to_string());
        let method = req.method().to_string();
        let user_agent = if let Some(ua) = req.headers().get(header::USER_AGENT) {
            let s = ua.to_str().unwrap_or("");
            Some(s.to_string())
        } else {
            None
        };
        Self { user, ip_address, method, user_agent, uri }
    }
}