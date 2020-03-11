use crate::db::*;
use crate::errors::BigNeonError;
use crate::server::GetAppState;
use actix_web::{FromRequest, HttpRequest, Result, dev::Payload};
use diesel;
use diesel::connection::TransactionManager;
use diesel::Connection as DieselConnection;
use diesel::PgConnection;
use std::sync::Arc;
use futures::future::{Ready, ok, err};

pub struct ReadonlyConnection {
    inner: Arc<ConnectionType>,
}

impl From<ConnectionType> for ReadonlyConnection {
    fn from(connection_type: ConnectionType) -> Self {
        ReadonlyConnection {
            inner: Arc::new(connection_type),
        }
    }
}

impl From<PgConnection> for ReadonlyConnection {
    fn from(connection: PgConnection) -> Self {
        ConnectionType::Pg(connection).into()
    }
}

impl From<Connection> for ReadonlyConnection {
    fn from(connection: Connection) -> Self {
        ReadonlyConnection {
            inner: connection.inner,
        }
    }
}

//impl From<ReadonlyConnection> for Connection {
//    fn from(readonly_connection: ReadonlyConnection) -> Self {
//        let pg_connection = readonly_connection.clone().get();
//        let connection = pg_connection.into();
//        connection
//    }
//}

impl ReadonlyConnection {
    pub fn get(&self) -> &PgConnection {
        match *self.inner {
            ConnectionType::Pg(ref connection) => connection,
            ConnectionType::R2D2(ref connection) => connection,
        }
    }

    pub fn commit_transaction(&self) -> Result<(), diesel::result::Error> {
        Ok(())
    }

    pub fn begin_transaction(&self) -> Result<(), diesel::result::Error> {
        Ok(())
    }

    pub fn rollback_transaction(&self) -> Result<(), diesel::result::Error> {
        Ok(())
    }
}

impl Clone for ReadonlyConnection {
    fn clone(&self) -> Self {
        ReadonlyConnection {
            inner: self.inner.clone(),
        }
    }
}

impl FromRequest for ReadonlyConnection {
    type Config = ();
    type Error = BigNeonError;
    type Future = Ready<Result<ReadonlyConnection, Self::Error>>;

    fn from_request(request: &HttpRequest, _: &mut Payload) -> Self::Future {
        if let Some(connection) = request.extensions().get::<ReadonlyConnection>() {
            return ok(connection.clone());
        }

        // should be moved to web::block, but would require Connection to be Sync
        let connection = match request.state().database_ro.get_ro_connection() {
            Ok(conn) => conn,
            Err(e) => return err(e.into()),
        };
        {
            let connection_object = connection.get();
            if let Err(e) = connection_object
                .transaction_manager()
                .begin_transaction(connection_object) {
                    return err(e.into());
                }
        }

        request.extensions_mut().insert(connection.clone());
        ok(connection)
    }
}
