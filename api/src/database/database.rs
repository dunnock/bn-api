use crate::config::Config;
use crate::database::{CacheDatabase, ConnectionType};
use crate::database::{Connection, ReadonlyConnection};
use ::r2d2::Error as R2D2Error;
use cache::RedisAsyncPool;
use diesel::r2d2::{self, ConnectionManager};
use diesel::PgConnection;

type R2D2Pool = r2d2::Pool<ConnectionManager<PgConnection>>;

pub struct Database {
    connection_pool: R2D2Pool,
    pub cache_database: CacheDatabase,
}

impl Database {
    pub async fn from_config(config: &Config) -> Database {
        Database {
            connection_pool: create_connection_pool(&config, config.database_url.clone()),
            cache_database: CacheDatabase {
                inner: load_redis_connection(config).await,
            },
        }
    }

    pub async fn readonly_from_config(config: &Config) -> Database {
        Database {
            connection_pool: create_connection_pool(&config, config.readonly_database_url.clone()),
            cache_database: CacheDatabase {
                inner: load_redis_connection(config).await,
            },
        }
    }

    pub fn get_connection(&self) -> Result<Connection, R2D2Error> {
        let conn = self.connection_pool.get()?;
        Ok(ConnectionType::R2D2(conn).into())
    }

    pub fn get_ro_connection(&self) -> Result<ReadonlyConnection, R2D2Error> {
        let conn = self.connection_pool.get()?;
        Ok(ConnectionType::R2D2(conn).into())
    }
}

impl Clone for Database {
    fn clone(&self) -> Self {
        Database {
            connection_pool: self.connection_pool.clone(),
            cache_database: self.cache_database.clone(),
        }
    }
}

fn create_connection_pool(config: &Config, database_url: String) -> R2D2Pool {
    let r2d2_config = r2d2::Pool::builder()
        .min_idle(Some(config.connection_pool.min))
        .max_size(config.connection_pool.max);

    let connection_manager = ConnectionManager::new(database_url);

    r2d2_config
        .build(connection_manager)
        .expect("Failed to create connection pool.")
}

pub async fn load_redis_connection(config: &Config) -> Option<RedisAsyncPool> {
    match config.redis {
        Some(ref redis) => Some(
            RedisAsyncPool::from_config(redis)
                .await
                .expect("Redis failed to create connection pool"),
        ),
        None => None,
    }
}
