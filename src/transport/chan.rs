//! Implements PayloadRead and PayloadWrite for mpsc channels

use async_trait::async_trait;
use futures::channel::mpsc::{UnboundedReceiver, UnboundedSender};

use crate::error::Error;
use super::{PayloadRead, PayloadWrite};

#[async_trait]
impl PayloadRead for UnboundedReceiver<Vec<u8>> {
    async fn read_payload(&mut self) -> Option<Result<Vec<u8>, Error>> {
        unimplemented!()
    }
}

#[async_trait]
impl PayloadWrite for UnboundedSender<Vec<u8>> {
    async fn write_payload(&mut self, payload: Vec<u8>) -> Result<(), Error> {
        unimplemented!()
    }
}