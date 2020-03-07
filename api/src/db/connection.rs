use actix_web::{FromRequest, HttpRequest, Result};
use crate::db::*;
use diesel;
use diesel::connection::TransactionManager;
use diesel::Connection as DieselConnection;
use diesel::PgConnection;
use crate::errors::BigNeonError;
use crate::server::AppState;
use std::sync::Arc;

pub struct Connection {
    pub inner: Arc<ConnectionType>,
}

impl From<ConnectionType> for Connection {
    fn from(connection_type: ConnectionType) -> Self {
        Connection {
            inner: Arc::new(connection_type),
        }
    }
}

impl From<PgConnection> for Connection {
    fn from(connection: PgConnection) -> Self {
        ConnectionType::Pg(connection).into()
    }
}

impl Connection {
    pub fn get(&self) -> &PgConnection {
        match *self.inner {
            ConnectionType::Pg(ref connection) => connection,
            ConnectionType::R2D2(ref connection) => connection,
        }
    }

    pub fn commit_transaction(&self) -> Result<(), diesel::result::Error> {
        self.get().transaction_manager().commit_transaction(self.get())
    }

    pub fn begin_transaction(&self) -> Result<(), diesel::result::Error> {
        self.get().transaction_manager().begin_transaction(self.get())
    }

    pub fn rollback_transaction(&self) -> Result<(), diesel::result::Error> {
        self.get().transaction_manager().rollback_transaction(self.get())
    }
}

impl Clone for Connection {
    fn clone(&self) -> Self {
        Connection {
            inner: self.inner.clone(),
        }
    }
}

impl FromRequest<AppState> for Connection {
    type Config = ();
    type Result = Result<Connection, BigNeonError>;

    fn from_request(request: &HttpRequest<AppState>, _config: &Self::Config) -> Self::Result {
        if let Some(connection) = request.extensions().get::<Connection>() {
            return Ok(connection.clone());
        }

        let connection = request.state().database.get_connection()?;
        {
            let connection_object = connection.get();
            connection_object
                .transaction_manager()
                .begin_transaction(connection_object)?;
        }

        request.extensions_mut().insert(connection.clone());
        Ok(connection)
    }
}
