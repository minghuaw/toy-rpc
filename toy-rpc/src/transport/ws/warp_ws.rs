//! WebSocket support for `warp`
//! Separate implementation is required because `warp` has wrapped `tungstenite` types
use super::*;
use warp::ws::{Message, WebSocket};

#[async_trait]
impl PayloadRead for StreamHalf<SplitStream<WebSocket>, CanSink> {
    async fn read_payload(&mut self) -> Option<Result<Vec<u8>, IoError>> {
        let msg = self.next().await?;
        match msg {
            Err(e) => {
                return Some(Err(std::io::Error::new(
                    ErrorKind::InvalidData,
                    e.to_string(),
                )))
            }
            Ok(m) => {
                if m.is_close() {
                    return None;
                } else if m.is_binary() {
                    return Some(Ok(m.into_bytes()));
                }
                Some(Err(std::io::Error::new(
                    ErrorKind::InvalidData,
                    "Expecting WebSocket::Message::Binary",
                )))
            }
        }
    }
}

#[async_trait]
impl PayloadWrite for SinkHalf<SplitSink<WebSocket, Message>, CanSink> {
    async fn write_payload(&mut self, payload: &[u8]) -> Result<(), IoError> {
        let msg = Message::binary(payload);

        // FIXME: `warp` has wrapped all errors into a trait object and doesn't
        // provide public API to retrieve the original error.
        self.send(msg).await.map_err(|e| into_io_err_other(&e))
    }
}

#[async_trait]
impl GracefulShutdown for SinkHalf<SplitSink<WebSocket, Message>, CanSink> {
    async fn close(&mut self) {
        let msg = Message::close();

        if let Err(err) = self.send(msg).await {
            let err_str = format!("{}", err);
            if err_str != CONNECTION_CLOSED_ERR_STR {
                log::error!("{}", err_str)
            }
        }
    }
}
