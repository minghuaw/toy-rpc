// pub use toy_rpc_definitions::{message};

pub use toy_rpc_macros as macros;

mod codec;
mod transport;
mod message;

pub mod error;
pub mod service;
pub mod client;
pub mod server;

// re-export
pub use erased_serde;
pub use lazy_static;
