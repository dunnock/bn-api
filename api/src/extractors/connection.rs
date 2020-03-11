use crate::server::AppState;
use actix_web::{FromRequest, HttpRequest, Result};
use bigneon_db_connections::{Connection, ReadonlyConnection, CacheDatabase};
use diesel::connection::TransactionManager;
use diesel::Connection as DieselConnection;

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

impl FromRequest<AppState> for ReadonlyConnection {
    type Config = ();
    type Result = Result<ReadonlyConnection, BigNeonError>;

    fn from_request(request: &HttpRequest<AppState>, _config: &Self::Config) -> Self::Result {
        if let Some(connection) = request.extensions().get::<ReadonlyConnection>() {
            return Ok(connection.clone());
        }

        let connection = request.state().database_ro.get_ro_connection()?;
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

impl FromRequest<AppState> for CacheDatabase {
    type Config = ();
    type Result = Result<CacheDatabase, BigNeonError>;

    fn from_request(request: &HttpRequest<AppState>, _config: &Self::Config) -> Self::Result {
        if let Some(connection) = request.extensions().get::<CacheDatabase>() {
            return Ok(connection.clone());
        }

        let connection = request.state().database.cache_database.clone();

        request.extensions_mut().insert(connection.clone());
        Ok(connection)
    }
}
