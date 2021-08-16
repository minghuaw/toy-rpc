//! Integration with axum using WebSocket
//! A separate implementation is required because `axum` has wrapped `tungstenite` types

use super::*;
use axum::ws::{Message, WebSocket};

#[async_trait]
impl PayloadRead for StreamHalf<SplitStream<WebSocket>, CanSink> {
    async fn read_payload(&mut self) -> Option<Result<Vec<u8>, Error>> {
        match self.next().await? {
            Err(e) => {
                return Some(Err(Error::IoError(std::io::Error::new(
                    ErrorKind::InvalidData,
                    e.to_string(),
                ))))
            }
            Ok(m) => {
                if m.is_close() {
                    return None;
                } else if m.is_binary() {
                    return Some(Ok(m.into_bytes()));
                }
                Some(Err(Error::IoError(std::io::Error::new(
                    ErrorKind::InvalidData,
                    "Expecting WebSocket::Message::Binary, but found something else".to_string(),
                ))))
            }
        }
    }
}

#[async_trait]
impl PayloadWrite for SinkHalf<SplitSink<WebSocket, Message>, CanSink> {
    async fn write_payload(&mut self, payload: &[u8]) -> Result<(), Error> {
        let msg = Message::binary(payload);

        self.send(msg).await.map_err(|e| Error::Internal(e))
    }
}

#[async_trait]
impl GracefulShutdown for SinkHalf<SplitSink<WebSocket, Message>, CanSink> {
    async fn close(&mut self) {
        let msg = Message::Close(None);

        if let Err(err) = self.send(msg).await {
            match err {
                tungstenite::Error::ConnectionClosed => { },
                tungstenite::Error::AlreadyClosed => { },
                e @ _ => {
                    log::error!("{}", e)
                }
            }
        }
    }
}
