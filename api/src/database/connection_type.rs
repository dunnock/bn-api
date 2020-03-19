use diesel::r2d2::{self, ConnectionManager};
use diesel::PgConnection;

type R2D2PooledConnection = r2d2::PooledConnection<ConnectionManager<PgConnection>>;

pub enum ConnectionType {
    Pg(PgConnection),
    R2D2(R2D2PooledConnection),
}
