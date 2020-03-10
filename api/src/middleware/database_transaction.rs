use crate::db::Connection;
use crate::errors::BigNeonError;
use actix_web::error;
use actix_web::{FromRequest, HttpRequest};
use diesel::connection::TransactionManager;
use diesel::Connection as DieselConnection;
use actix_web::dev::{ServiceRequest, ServiceResponse, MessageBody};
use actix_service::Service;
use std::error::Error;
use std::pin::Pin;
use std::future::Future;

pub trait RequestConnection {
    fn connection(&self) -> error::Result<Connection>;
}

impl RequestConnection for HttpRequest {
    fn connection(&self) -> error::Result<Connection> {
        Ok(Connection::from_request(&self, &())?)
    }
}

pub struct DatabaseTransaction {}

impl DatabaseTransaction {
    pub fn new() -> DatabaseTransaction {
        DatabaseTransaction {}
    }
}

impl DatabaseTransaction {
    pub fn create<S, B>() -> impl FnMut(ServiceRequest, &mut S) -> Pin<Box<dyn Future<Output = error::Result<ServiceResponse<B>>> + 'static>>
    where
        S: Service<Request = ServiceRequest, Response = ServiceResponse<B>, Error = error::Error>,
        S::Future: 'static,
        B: MessageBody,
    {
        |sreq, serv| {
            let srv_fut = serv.call(sreq);
            Box::pin(async move {
                let resp = srv_fut.await?;
                Self::response(resp.request(), resp)
            })
        }
    }

    fn response<B>(request: &HttpRequest, response: ServiceResponse<B>) -> error::Result<ServiceResponse<B>> {
        if let Some(connection) = request.extensions().get::<Connection>() {
            let connection_object = connection.get();

            let transaction_response = match response.error() {
                Some(_) => connection_object
                    .transaction_manager()
                    .rollback_transaction(connection_object),
                None => connection_object
                    .transaction_manager()
                    .commit_transaction(connection_object),
            };

            match transaction_response {
                Ok(_) => (),
                Err(e) => {
                    error!("Diesel Error: {}", e.description());
                    let error: BigNeonError = e.into();
                    return Err(error);
                }
            }
        };

        Ok(response)
    }
}
