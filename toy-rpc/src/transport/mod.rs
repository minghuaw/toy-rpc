//! Custom binary transport and WebSocket integration

use async_trait::async_trait;

use crate::error::Error;

#[cfg(all(
    any(
        feature = "serde_bincode",
        feature = "serde_cbor",
        feature = "serde_rmp"
    ),
    any(feature = "async_std_runtime", feature = "tokio_runtime",)
))]
pub(crate) mod frame;

pub(crate) mod ws;

/// Reads bytes from transport protocols that carry payload (ie. WebSocket)
#[async_trait]
pub trait PayloadRead {
    /// Reads bytes from the payload
    async fn read_payload(&mut self) -> Option<Result<Vec<u8>, Error>>;
}

/// Writes bytes as payload on transport protocols that carry payload (ie. WebSocket)
#[async_trait]
pub trait PayloadWrite {
    /// Writes bytes to the payload
    async fn write_payload(&mut self, payload: Vec<u8>) -> Result<(), Error>;
}
