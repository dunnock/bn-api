#![deny(unreachable_patterns)]
#![deny(unknown_lints)]
#![cfg_attr(not(debug_assertions), deny(unused_variables))]
#![cfg_attr(not(debug_assertions), deny(unused_imports))]
#![cfg_attr(not(debug_assertions), deny(dead_code))]
// Unused results is more often than not an error
#![deny(unused_must_use)]
#![cfg_attr(not(debug_assertions), deny(unused_extern_crates))]
#[macro_use]
extern crate db;
extern crate expo_server_sdk as expo;
extern crate jsonwebtoken as jwt;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate log;
#[macro_use]
extern crate logging;
#[macro_use]
extern crate macros;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate serde_json;
#[macro_use]
extern crate validator_derive;

pub mod auth;
pub mod communications;
pub mod config;
pub mod controllers;
pub mod database;
pub mod domain_events;
pub mod errors;
pub mod extractors;
pub mod helpers;
pub mod middleware;
pub mod models;
mod payments;
mod routing;
pub mod server;
pub mod utils;
