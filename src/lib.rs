// Copyright (c) 2018-2022, agnos.ai UK Ltd, all rights reserved.
//---------------------------------------------------------------
#![feature(alloc_c_string)]
#![feature(rustc_private)]

extern crate core;

use core::str::FromStr;

use lazy_static::lazy_static;
pub use mime::Mime;

pub use cursor::Cursor;
pub use data_store_connection::DataStoreConnection;
pub use exception::Error;
pub use graph::Graph;
pub use parameters::Parameters;
pub use prefixes::Prefixes;
pub use role_creds::RoleCreds;
pub use server::Server;
pub use server_connection::Connection;
pub use statement::Statement;
pub use transaction::Transaction;

lazy_static! {
    pub static ref TEXT_TURTLE: Mime = Mime::from_str("text/turtle").unwrap();
}

mod cursor;
mod data_store_connection;
mod exception;
mod graph;
mod parameters;
mod prefixes;
mod role_creds;
mod server;
mod server_connection;
mod statement;
mod tests;
mod transaction;

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
