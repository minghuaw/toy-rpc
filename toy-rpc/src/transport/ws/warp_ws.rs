//! WebSocket support for `warp`
//! Separate implementation is required because `warp` has wrapped `tungstenite` types
use super::*;
use warp::ws::{Message as WsMessage, WebSocket};

#[async_trait]
impl PayloadRead for StreamHalf<SplitStream<WebSocket>, CanSink> {
    async fn read_payload(&mut self) -> Option<Result<Vec<u8>, Error>> {
        let msg = self.next().await?;
        match msg {
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
impl PayloadWrite for SinkHalf<SplitSink<WebSocket, WsMessage>, CanSink> {
    async fn write_payload(&mut self, payload: &[u8]) -> Result<(), Error> {
        let msg = warp::ws::Message::binary(payload);

        self.send(msg)
            .await
            .map_err(|e| Error::Internal(Box::new(e)))
    }
}

#[async_trait]
impl GracefulShutdown for SinkHalf<SplitSink<WebSocket, WsMessage>, CanSink> {
    async fn close(&mut self) {
        let msg = warp::ws::Message::close();

        self.send(msg)
            .await
            .map_err(|err| Error::Internal(Box::new(err)))
            .unwrap_or_else(|err| log::error!("{}", err))
    }
}
