//#![deny(unused_extern_crates)]
extern crate api;
extern crate dotenv;
#[macro_use]
extern crate log;
#[macro_use]
extern crate logging;
#[macro_use]
extern crate serde_json;
extern crate clap;

use api::config::Config;
use api::server::Server;
use clap::*;
use dotenv::dotenv;
use log::Level::*;

#[actix_rt::main]
async fn main() {
    logging::setup_logger();
    info!("Loading environment");
    dotenv().ok();
    let environment = Config::parse_environment().unwrap_or_else(|_| panic!("Environment is invalid."));
    jlog!(Info, &format!("Environment loaded: {:?}", environment));

    let config = Config::new(environment);

    let matches = App::new("Big Neon API Server")
        .author("Big Neon")
        .version(crate_version!())
        .about("HTTP REST API server and event and scheduled tasks processor")
        .arg(
            Arg::with_name("process-actions")
                .help("Fetches and processes actions from the database")
                .short("t")
                .default_value("true"),
        )
        .arg(
            Arg::with_name("process-events")
                .help("Fetches and processes events from the database")
                .short("e")
                .default_value("true"),
        )
        .arg(
            Arg::with_name("process-http")
                .help("Runs an HTTP API server processing requests")
                .short("a")
                .default_value("true"),
        )
        .arg(
            Arg::with_name("process-redis-pubsub")
                .help("Processes redis pub subs")
                .short("r")
                .default_value("true"),
        )
        .arg(
            Arg::with_name("run-til-empty")
                .help("Process all pending domain actions and then exits")
                .short("b")
                .default_value("false"),
        )
        .get_matches();

    jlog!(Info, "Starting server", {"app_name": config.app_name});
    Server::start(
        config,
        matches
            .value_of("process-actions")
            .unwrap_or("true")
            .parse()
            .expect("Unknown value for `process-actions`. Use `true` or `false`"),
        matches
            .value_of("process-events")
            .unwrap_or("true")
            .parse()
            .expect("Unknown value for `process-events`. Use `true` or `false`"),
        matches
            .value_of("process-http")
            .unwrap_or("true")
            .parse()
            .expect("Unknown value for `process-http`. Use `true` or `false`"),
        matches
            .value_of("process-redis-pubsub")
            .unwrap_or("true")
            .parse()
            .expect("Unknown value for `process-redis-pubsub`. Use `true` or `false`"),
        matches
            .value_of("run-til-empty")
            .unwrap_or("false")
            .parse()
            .expect("Unknown value for `run-til-empty`. Use `true` or `false`"),
    )
    .await;
    info!("Server shutting down");
}
