// Quiet diesel warnings https://github.com/diesel-rs/diesel/issues/1785
#![allow(proc_macro_derive_resolution_fallback)]
// Force these as errors so that they are not lost in all the diesel warnings
#![deny(unreachable_patterns)]
#![deny(unknown_lints)]
#![deny(unused_variables)]
#![deny(unused_imports)]
// Unused results is more often than not an error
#![deny(unused_must_use)]
#![deny(unused_extern_crates)]
#![deny(dead_code)]
#![recursion_limit = "256"]
#[macro_use]
extern crate diesel;
extern crate argon2rs;
extern crate backtrace;
extern crate bigneon_http;
extern crate chrono;
extern crate chrono_tz;
extern crate diesel_migrations;
extern crate hex;
extern crate itertools;
extern crate jsonwebtoken;
//#[macro_use]
extern crate log;
#[macro_use]
extern crate logging;

// This crate is only used in tests at the moment
#[allow(unused_extern_crates)]
#[cfg_attr(test, macro_use)]
extern crate macros;

extern crate rand;
extern crate regex;
extern crate ring;
#[macro_use]
extern crate embed_dirs_derive;
extern crate uuid;
#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate serde_with;
#[macro_use]
extern crate serde_json;
#[macro_use]
extern crate validator_derive;
extern crate tari_client;
extern crate unidecode;
extern crate url;
extern crate validator;

pub mod models;
pub mod schema;
pub mod services;
pub mod utils;
pub mod validators;

//#[cfg(test)]
mod test;

//#[cfg(test)]
pub mod dev {
    pub use test::*;
}

pub mod prelude {
    pub use models::*;
    pub use services::*;
    pub use utils::errors::*;
    pub use utils::*;
}
