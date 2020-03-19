// Quiet diesel warnings https://github.com/diesel-rs/diesel/issues/1785
#![allow(proc_macro_derive_resolution_fallback)]
#![deny(unreachable_patterns)]
#![deny(unknown_lints)]
#![cfg_attr(not(debug_assertions), deny(unused_variables))]
#![cfg_attr(not(debug_assertions), deny(unused_imports))]
#![cfg_attr(not(debug_assertions), deny(dead_code))]
// Unused results is more often than not an error
#![deny(unused_must_use)]
#![deny(unused_extern_crates)]
extern crate bigneon_db as db;
extern crate chrono;
extern crate chrono_tz;
extern crate diesel;
extern crate rand;
#[macro_use]
extern crate serde_json;
extern crate uuid;
extern crate validator;
#[macro_use]
extern crate macros;
extern crate itertools;
extern crate tari_client;

mod unit;
