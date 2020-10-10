pub use toy_rpc_definitions::{message, service, error};

pub use toy_rpc_macros as macros;

mod codec;
mod transport;

pub mod client;
pub mod server;

// re-export
pub use ::erased_serde;
pub use ::lazy_static;
