use crate::extractors::Uuid;
use actix_web::error;
use actix_web::http::header;
use actix_web::http::StatusCode;
use actix_web::FromRequest;
use actix_web::dev::{ServiceRequest, ServiceResponse, MessageBody};
use actix_service::Service;
use log::Level;
use std::future::Future;
use std::pin::Pin;

pub struct BigNeonLogger;

impl BigNeonLogger {
    pub fn create<S, B>() -> impl FnMut(ServiceRequest, &mut S) -> Pin<Box<dyn Future<Output = error::Result<ServiceResponse<B>>> + 'static>>
    where
        S: Service<Request = ServiceRequest, Response = ServiceResponse<B>, Error = error::Error>,
        S::Future: 'static,
        B: MessageBody,
    {
        |sreq, serv| {
            let data = RequestLogData::from(&sreq);
            BigNeonLogger::start(&data);
            let srv_fut = serv.call(sreq);
            Box::pin(async move {
                let resp = srv_fut.await;
                BigNeonLogger::finish(&data, &resp);
                resp
            })
        }
    }

    // log message at the start of request lifecycle
    fn start(data: &RequestLogData) -> RequestLogData {
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
    }

    // log message at the end of request lifecycle
    fn finish<B: MessageBody>(data: &RequestLogData, resp: &error::Result<ServiceResponse<B>>) {
        if let Err(error) =  resp {
            let level = if resp.status() == StatusCode::UNAUTHORIZED {
                Level::Info
            } else if resp.status().is_client_error() {
                Level::Warn
            } else {
                Level::Error
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
        }
    }
}


struct RequestLogData {
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