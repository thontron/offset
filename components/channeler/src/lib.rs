#![crate_type = "lib"] 
#![feature(futures_api, async_await, await_macro, arbitrary_self_types)]
#![feature(nll)]
#![feature(try_from)]
#![feature(generators)]
#![feature(never_type)]

mod channeler;
mod types;
mod transform_pool;
mod listen_pool_state;
mod listen_pool;
mod connect_pool;
mod connector_utils;
mod overwrite_channel;
mod spawn;

#[macro_use]
extern crate log;

pub use self::channeler::ChannelerError;
pub use self::spawn::{spawn_channeler, SpawnChannelerError};
