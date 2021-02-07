use async_trait::async_trait;

use crate::error::Error;
use crate::GracefulShutdown;

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

#[async_trait]
pub trait PayloadRead {
    async fn read_payload(&mut self) -> Option<Result<Vec<u8>, Error>>;
}

#[async_trait]
pub trait PayloadWrite: GracefulShutdown {
    async fn write_payload(&mut self, payload: Vec<u8>) -> Result<(), Error>;
}
