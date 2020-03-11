use crate::*;
use diesel;
use diesel::PgConnection;
use std::sync::Arc;

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
