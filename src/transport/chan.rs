//! Implements PayloadRead and PayloadWrite for mpsc channels
//! 

use std::unimplemented;

use async_trait::async_trait;
use futures::{SinkExt, StreamExt};
use futures::{Stream, Sink};

// use crate::error::Error;
// use super::{PayloadRead, PayloadWrite};

// #[async_trait]
// impl<S: Stream<Item=Vec<u8>> + Unpin + Send> PayloadRead for S {
//     async fn read_payload(&mut self) -> Option<Result<Vec<u8>, Error>> {
//         Ok(self.next().await)
//             .transpose()
//     }
// }

// #[async_trait]
// impl<E: std::error::Error + 'static, S: Sink<Vec<u8>, Error=E> + Unpin + Send> PayloadWrite for S {
//     async fn write_payload(&mut self, payload: Vec<u8>) -> Result<(), Error> {
//         self.send(payload).await
//             .map_err(|e| Error::TransportError{msg: e.to_string()})
//     }
// }

// #[async_trait]
// impl PayloadRead for tokio_stream::wrappers::UnboundedReceiverStream<Vec<u8>> {
//     async fn read_payload(&mut self) -> Option<Result<Vec<u8>, Error>> {
//         unimplemented!()
//     }
// }

// #[async_trait]
// impl PayloadWrite for tokio::sync::mpsc::UnboundedSender<Vec<u8>> {
//     async fn write_payload(&mut self, payload: Vec<u8>) -> Result<(), Error> {
//         self.send(payload)
//             .map_err(|e| Error::TransportError{msg: e.to_string()})
//     }
// }