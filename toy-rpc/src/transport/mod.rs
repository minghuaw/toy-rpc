//! Custom binary transport and WebSocket integration

use async_trait::async_trait;

use crate::error::IoError;

#[cfg(all(
    any(
        feature = "serde_bincode",
        feature = "serde_cbor",
        feature = "serde_rmp"
    ),
    any(feature = "async_std_runtime", feature = "tokio_runtime",)
))]
pub(crate) mod frame;

#[cfg(any(feature = "ws_tokio", feature = "ws_async_std"))]
pub(crate) mod ws;

#[cfg(any(
    all(
        any(
            feature = "serde_bincode",
            feature = "serde_cbor",
            feature = "serde_rmp"
        ),
        any(feature = "async_std_runtime", feature = "tokio_runtime",)
    ),
    any(feature = "ws_tokio", feature = "ws_async_std")
))]
pub(crate) fn as_io_err_other(err: &impl std::fmt::Display) -> IoError {
    let msg = format!("{}", err);
    std::io::Error::new(std::io::ErrorKind::Other, msg)
}

/// Reads bytes from transport protocols that carry payload (ie. WebSocket)
#[async_trait]
pub trait PayloadRead {
    /// Reads bytes from the payload
    async fn read_payload(&mut self) -> Option<Result<Vec<u8>, IoError>>;
}

/// Writes bytes as payload on transport protocols that carry payload (ie. WebSocket)
#[async_trait]
pub trait PayloadWrite {
    /// Writes bytes to the payload
    async fn write_payload(&mut self, payload: &[u8]) -> Result<(), IoError>;
}
