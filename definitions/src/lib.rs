mod error;

pub mod message;

#[cfg(not(feature="async_service"))]
pub mod sync_service;

#[cfg(feature="async_service")]
pub mod async_service;

// re-export
#[cfg(not(feature="async_service"))]
pub use sync_service as service;

#[cfg(feature="async_service")]
pub use async_service as service;

pub use error::{Error, RpcError};
