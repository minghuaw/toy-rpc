use async_trait::async_trait;

use crate::error::Error;

#[cfg(any(
    feature = "serde_bincode",
    feature = "serde_cbor",
    feature = "serde_rmp"
))]
pub(crate) mod frame;

pub(crate) mod ws;

pub(crate) mod chan;

#[async_trait]
pub trait PayloadRead {
    async fn read_payload(&mut self) -> Option<Result<Vec<u8>, Error>>;
}

#[async_trait]
pub trait PayloadWrite {
    async fn write_payload(&mut self, payload: Vec<u8>) -> Result<(), Error>;
}