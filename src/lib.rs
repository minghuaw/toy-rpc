//! A toy RPC crate that mimics the golang's "net/rpc" package
//! 
//! ## Documentation
//! 
//! ## Crate features
//! 
//! This crate offers the following features tags
//! 
//! - `std`: enables `serde/std`
//! - `bincode`: the default codec will use `bincode` 
//! for serialization/deserialization
//! - `serde_json`: the default codec will use `serde_json` 
//! for `json` serialization/deserialization
//! 
//! 


pub use toy_rpc_macros as macros;

pub mod codec;
pub mod message;
pub mod transport;

pub mod client;
pub mod error;
pub mod server;
pub mod service;

// re-export
pub use erased_serde;
pub use lazy_static;
