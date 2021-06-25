pub mod codec;
pub mod error;
pub mod message;
pub mod service;
pub mod transport;
pub mod util;

#[cfg(any(
    feature = "tide-websockets",
    feature = "warp",
    feature = "actix",
    feature = "client",
))]
pub const DEFAULT_RPC_PATH: &str = "_rpc_";