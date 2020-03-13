use crate::extractors::Uuid;
use actix_service::Service;
use actix_web::http::{header, StatusCode};
use actix_web::{dev, error};
use futures::future::{ok, Ready};
use log::Level;

pub struct BigNeonLogger;

impl BigNeonLogger {
    pub fn new() -> Self {
        Self {}
    }

    // log message at the start of request lifecycle
    pub fn start(sreq: &dev::ServiceRequest) -> RequestLogData {
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
    pub fn finish<B>(
        data: &RequestLogData,
        resp: error::Result<dev::ServiceResponse<B>>,
    ) -> error::Result<dev::ServiceResponse<B>> {
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
    fn from(req: &dev::ServiceRequest) -> Self {
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
        Self {
            user,
            ip_address,
            method,
            user_agent,
            uri,
        }
    }
}

impl<S, B> dev::Transform<S> for BigNeonLogger
where
    S: Service<Request = dev::ServiceRequest, Response = dev::ServiceResponse<B>, Error = error::Error> + 'static,
    B: dev::MessageBody,
{
    type Request = S::Request;
    type Response = S::Response;
    type Error = S::Error;
    type InitError = ();
    type Transform = LoggerService<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(LoggerService::new(service))
    }
}

use std::cell::RefCell;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll};

pub struct LoggerService<S> {
    service: Rc<RefCell<S>>,
}

impl<S> LoggerService<S> {
    fn new(service: S) -> Self {
        Self {
            service: Rc::new(RefCell::new(service)),
        }
    }
}

impl<S, B> Service for LoggerService<S>
where
    S: Service<Request = dev::ServiceRequest, Response = dev::ServiceResponse<B>, Error = error::Error> + 'static,
    B: dev::MessageBody,
{
    type Request = S::Request;
    type Response = S::Response;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.service.borrow_mut().poll_ready(cx).map_err(error::Error::from)
    }

    fn call(&mut self, request: Self::Request) -> Self::Future {
        let service = self.service.clone();
        Box::pin(async move {
            let data = BigNeonLogger::start(&request);
            let fut = service.borrow_mut().call(request);
            let response = fut.await;
            BigNeonLogger::finish(&data, response)
        })
    }
}
