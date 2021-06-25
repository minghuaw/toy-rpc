//! WebSocket support for `warp`

use super::*;

#[async_trait]
impl<S, E> PayloadRead for S
where
    S: Stream<Item = Result<warp::ws::Message, E>> + Send + Sync + Unpin,
    E: std::error::Error + 'static,
{
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
impl<S, E> PayloadWrite for S
where
    S: Sink<warp::ws::Message, Error = E> + Send + Sync + Unpin,
    E: std::error::Error + Send + Sync + 'static,
{
    async fn write_payload(&mut self, payload: Vec<u8>) -> Result<(), Error> {
        // let msg = WsMessage::Binary(payload.into());
        let msg = warp::ws::Message::binary(payload);

        self.send(msg)
            .await
            .map_err(|e| Error::Internal(Box::new(e)))
    }
}
