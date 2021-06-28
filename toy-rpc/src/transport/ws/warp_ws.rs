//! WebSocket support for `warp`
use warp::ws::{WebSocket, Message as WsMessage};
use super::*;

#[async_trait]
impl PayloadRead for StreamHalf<SplitStream<WebSocket>, CanSink>
// where
//     S: Stream<Item = Result<warp::ws::Message, E>> + Send + Sync + Unpin,
//     E: std::error::Error + 'static,
{
    async fn read_payload(&mut self) -> Option<Result<Vec<u8>, Error>> {
        let msg = self.inner.next().await?;
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
impl PayloadWrite for SinkHalf<SplitSink<WebSocket, WsMessage>, CanSink>
{
    async fn write_payload(&mut self, payload: Vec<u8>) -> Result<(), Error> {
        // let msg = WsMessage::Binary(payload.into());
        let msg = warp::ws::Message::binary(payload);

        self.inner.send(msg)
            .await
            .map_err(|e| Error::Internal(Box::new(e)))
    }
}

#[async_trait]
impl GracefulShutdown for SinkHalf<SplitSink<WebSocket, WsMessage>, CanSink>
{
    async fn close(&mut self) {
        let msg = warp::ws::Message::close();

        self.inner.send(msg)
            .await
            .map_err(|err| Error::Internal(Box::new(err)))
            .unwrap_or_else(|err| log::error!("{}", err))
    }
}