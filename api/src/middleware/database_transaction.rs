use crate::db::Connection;
use crate::errors::BigNeonError;
use actix_web::dev::{Payload, ServiceResponse};
use actix_web::error;
use actix_web::{FromRequest, HttpRequest};
use diesel::connection::TransactionManager;
use diesel::Connection as DieselConnection;
use std::error::Error;

pub trait RequestConnection {
    fn connection(&self) -> error::Result<Connection>;
}

impl RequestConnection for HttpRequest {
    fn connection(&self) -> error::Result<Connection> {
        Ok(Connection::from_request(&self, &mut Payload::None).into_inner()?)
    }
}

pub struct DatabaseTransaction {}

impl DatabaseTransaction {
    pub fn new() -> DatabaseTransaction {
        DatabaseTransaction {}
    }
}

impl DatabaseTransaction {
    pub fn response<B>(response: ServiceResponse<B>) -> error::Result<ServiceResponse<B>> {
        let request = response.request();

        let res = if let Some(connection) = request.extensions().get::<Connection>() {
            let connection_object = connection.get();

            let transaction_response = match response.response().error() {
                Some(_) => connection_object
                    .transaction_manager()
                    .rollback_transaction(connection_object),
                None => connection_object
                    .transaction_manager()
                    .commit_transaction(connection_object),
            };

            match transaction_response {
                Ok(_) => Ok(()),
                Err(e) => {
                    error!("Diesel Error: {}", e.description());
                    let error: BigNeonError = e.into();
                    Err(error)
                }
            }
        } else {
            Ok(())
        };

        match res {
            Ok(_) => Ok(response),
            Err(err) => Ok(response.error_response(err)),
        }
    }
}
