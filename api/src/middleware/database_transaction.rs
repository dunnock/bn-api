use actix_web::error::Error as ActixWebError;
use actix_web::middleware::{Middleware, Response};
use actix_web::{FromRequest, HttpRequest, HttpResponse, ResponseError, Result};
use crate::db::Connection;
use diesel::connection::TransactionManager;
use diesel::Connection as DieselConnection;

pub trait RequestConnection {
    fn connection(&self) -> Result<Connection, ActixWebError>;
}

impl RequestConnection for HttpRequest {
    fn connection(&self) -> Result<Connection, ActixWebError> {
        Ok(Connection::from_request(&self, &())?)
    }
}

pub struct DatabaseTransaction {}
use crate::errors::BigNeonError;
use crate::server::AppState;
use std::error::Error;

impl DatabaseTransaction {
    pub fn new() -> DatabaseTransaction {
        DatabaseTransaction {}
    }
}

impl Middleware<AppState> for DatabaseTransaction {
    fn response(&self, request: &HttpRequest, response: HttpResponse) -> Result<Response> {
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
                    return Ok(Response::Done(error.error_response()));
                }
            }
        };

        Ok(Response::Done(response))
    }
}
