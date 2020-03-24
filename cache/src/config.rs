use std::time::Duration;

pub struct Config {
	pub database_url: String,
	pub read_timeout: Duration,
	pub write_timeout: Duration,
	pub max_size: usize,
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
