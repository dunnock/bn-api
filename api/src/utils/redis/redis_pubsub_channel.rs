use db::prelude::*;
use std::cmp::Ordering;
use std::fmt;
use std::str::FromStr;

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Debug, Eq, Hash)]
pub enum RedisPubSubChannel {
    TicketRedemptions,
}
string_enum! { RedisPubSubChannel[TicketRedemptions] }
