use std::time::Duration;

#[derive(Clone, Debug)]
/// Configuration for the cache
pub struct Config {
    /// Redis connection string, only host and port parts are supported atm
    pub database_url: String,
    /// Timeout for read operations
    pub read_timeout: Duration,
    /// Timeout for write operations
    pub write_timeout: Duration,
    /// Number of connections in the pool
    /// Default number of connections is number of CPUs on instance
    pub max_size: usize,
    /// Concurrency level for `GET` operations in the pool per connection
    /// `GET` operations will hold while there are >= `concurrency * max_size` GET operations in progress
    /// Default is 4 per connection
    pub concurrency: usize,
}

// safe enough and performant based on perf tests in benchmark, see README.md
const DEFAULT_CONCURRENCY: usize = 4;

impl Default for Config {
    fn default() -> Self {
        let max_size = num_cpus::get();
        Config {
            database_url: "redis://127.0.0.1/".to_owned(),
            read_timeout: Duration::from_millis(100),
            write_timeout: Duration::from_millis(100),
            max_size,
            concurrency: DEFAULT_CONCURRENCY,
        }
    }
}
