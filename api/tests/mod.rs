#![deny(unreachable_patterns)]
#![deny(unknown_lints)]
#![deny(unused_variables)]
#![deny(unused_imports)]
// Unused results is more often than not an error
#![deny(unused_must_use)]
#![deny(unused_extern_crates)]
#[macro_use]
extern crate macros;
#[macro_use]
extern crate serde_json;
#[macro_use]
extern crate serde_derive;
extern crate jsonwebtoken as jwt;

mod functional;
mod support;
mod unit;
