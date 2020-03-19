#![deny(unreachable_patterns)]
#![deny(unused_variables)]
#![deny(unused_imports)]
// Unused results is more often than not an error
#![deny(unused_must_use)]
#![deny(unused_parens)]
extern crate jsonrpc_core;
extern crate log;
#[macro_use]
extern crate logging;
extern crate reqwest;
extern crate serde;

extern crate serde_json;
#[macro_use]
extern crate serde_derive;
extern crate crypto;
extern crate hex;
extern crate rand;
extern crate secp256k1;
extern crate uuid;

mod cryptographic;
mod tari_client;
mod tari_error;
mod tari_messages;
mod tari_test_client;

pub use cryptographic::*;
pub use tari_client::HttpTariClient;
pub use tari_client::TariClient;
pub use tari_error::TariError;
pub use tari_messages::*;
pub use tari_test_client::TariTestClient;
