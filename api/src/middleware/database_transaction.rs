use crate::database::Connection;
use crate::errors::ApiError;
use actix_service::Service;
use actix_web::dev;
use actix_web::error;
use actix_web::{FromRequest, HttpRequest};
use diesel::connection::TransactionManager;
use diesel::Connection as DieselConnection;
use futures::future::{ok, Ready};

pub trait RequestConnection {
    fn connection(&self) -> Result<Connection, ApiError>;
}

impl RequestConnection for HttpRequest {
    fn connection(&self) -> Result<Connection, ApiError> {
        Connection::from_request(&self, &mut dev::Payload::None).into_inner()
    }
}

pub struct DatabaseTransaction;

impl DatabaseTransaction {
    pub fn new() -> DatabaseTransaction {
        DatabaseTransaction {}
    }

    // Reconcile reponse status and request's DB connection transaction
    pub fn complete<B>(response: dev::ServiceResponse<B>) -> error::Result<dev::ServiceResponse<B>> {
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
                    error!("Diesel Error: {}", e.to_string());
                    let error: ApiError = e.into();
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

impl<S, B> dev::Transform<S> for DatabaseTransaction
where
    S: Service<Request = dev::ServiceRequest, Response = dev::ServiceResponse<B>, Error = error::Error> + 'static,
    B: dev::MessageBody,
{
    type Request = S::Request;
    type Response = S::Response;
    type Error = S::Error;
    type InitError = ();
    type Transform = DatabaseTransactionService<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(DatabaseTransactionService::new(service))
    }
}

use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

pub struct DatabaseTransactionService<S> {
    service: S,
}

impl<S> DatabaseTransactionService<S> {
    fn new(service: S) -> Self {
        Self { service }
    }
}

impl<S, B> Service for DatabaseTransactionService<S>
where
    S: Service<Request = dev::ServiceRequest, Response = dev::ServiceResponse<B>, Error = error::Error> + 'static,
    B: dev::MessageBody,
{
    type Request = S::Request;
    type Response = S::Response;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.service.poll_ready(cx).map_err(error::Error::from)
    }

    fn call(&mut self, request: Self::Request) -> Self::Future {
        let fut = self.service.call(request);
        Box::pin(async move {
            let response = fut.await?;
            // In the case of error, connection to database
            // will be dropped and transaction will be rolled back
            // We still need to process correct response
            // and commit or rollback based on that
            DatabaseTransaction::complete(response)
        })
    }
}
