#![deny(unreachable_patterns)]
#![deny(unknown_lints)]
#![cfg_attr(not(debug_assertions), deny(unused_variables))]
#![cfg_attr(not(debug_assertions), deny(unused_imports))]
#![cfg_attr(not(debug_assertions), deny(dead_code))]
// Unused results is more often than not an error
#![deny(unused_must_use)]
#![cfg_attr(not(debug_assertions), deny(unused_extern_crates))]
extern crate actix;
extern crate actix_web;
#[macro_use]
extern crate bigneon_db;
extern crate bigneon_http;
extern crate branch_rs;
extern crate chrono;
extern crate customer_io;
extern crate diesel;
extern crate dotenv;
extern crate expo_server_sdk as expo;
extern crate facebook;
extern crate futures;
extern crate globee;
extern crate itertools;
extern crate jsonwebtoken as jwt;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate log;
#[macro_use]
extern crate logging;
#[macro_use]
extern crate macros;
extern crate phonenumber;
extern crate r2d2;
extern crate redis;
extern crate regex;
extern crate reqwest;
extern crate serde;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate serde_json;
extern crate serde_with;
extern crate stripe;
extern crate tari_client;
extern crate tokio;
extern crate twilio;
extern crate url;
extern crate uuid;
extern crate validator;
#[macro_use]
extern crate validator_derive;
extern crate cache;
extern crate sitemap;

pub use bigneon_errors as errors;
pub use bigneon_config as config;
pub use bigneon_domain_events as domain_events;
pub use bigneon_communications as communications;
pub use bigneon_db_connections as db;
pub use bigneon_payments as payments;

pub mod auth;
pub mod communications;
pub mod controllers;
pub mod extractors;
pub mod helpers;
pub mod middleware;
pub mod models;
mod routing;
pub mod server;
pub mod utils;
