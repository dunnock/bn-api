use crate::db::*;
use crate::errors::BigNeonError;
use crate::server::GetAppState;
use actix_web::{dev::Payload, FromRequest, HttpRequest, Result};
use diesel;
use diesel::connection::TransactionManager;
use diesel::Connection as DieselConnection;
use diesel::PgConnection;
use futures::future::{err, ok, Ready};
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

impl FromRequest for Connection {
    type Config = ();
    type Error = BigNeonError;
    type Future = Ready<Result<Connection, Self::Error>>;

    fn from_request(request: &HttpRequest, _: &mut Payload) -> Self::Future {
        if let Some(connection) = request.extensions().get::<Connection>() {
            return ok(connection.clone());
        }

        // should be moved to web::block, but would require Connection to be Sync
        let connection = match request.state().database.get_connection() {
            Ok(conn) => conn,
            Err(e) => return err(e.into()),
        };
        {
            let connection_object = connection.get();
            if let Err(e) = connection_object
                .transaction_manager()
                .begin_transaction(connection_object)
            {
                return err(e.into());
            }
        }

        request.extensions_mut().insert(connection.clone());
        ok(connection)
    }
}
