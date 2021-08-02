//! WebSocket support for `tide-websockets`

use tide_websockets as tide_ws;

use super::*;

impl WebSocketConn<tide_websockets::WebSocketConnection, CannotSink> {
    pub fn new_without_sink(inner: tide_websockets::WebSocketConnection) -> Self {
        Self {
            inner,
            can_sink: PhantomData,
        }
    }

    pub fn split(
        self,
    ) -> (
        SinkHalf<tide_ws::WebSocketConnection, CannotSink>,
        StreamHalf<tide_ws::WebSocketConnection, CannotSink>,
    ) {
        let writer = SinkHalf {
            inner: self.inner.clone(),
            can_sink: PhantomData,
        };
        let reader = StreamHalf {
            inner: self.inner,
            can_sink: PhantomData,
        };
        (writer, reader)
    }
}

#[async_trait]
impl PayloadRead for StreamHalf<tide_websockets::WebSocketConnection, CannotSink> {
    async fn read_payload(&mut self) -> Option<Result<Vec<u8>, Error>> {
        match self.inner.next().await? {
            Err(e) => {
                return Some(Err(Error::IoError(std::io::Error::new(
                    ErrorKind::InvalidData,
                    e.to_string(),
                ))))
            }
            Ok(msg) => {
                if let tide_websockets::Message::Binary(bytes) = msg {
                    return Some(Ok(bytes));
                } else if let tide_websockets::Message::Close(_) = msg {
                    return None;
                }

                Some(Err(Error::IoError(std::io::Error::new(
                    ErrorKind::InvalidData,
                    "Expecting WebSocket::Message::Binary",
                ))))
            }
        }
    }
}

#[async_trait]
impl PayloadWrite for SinkHalf<tide_websockets::WebSocketConnection, CannotSink> {
    async fn write_payload(&mut self, payload: &[u8]) -> Result<(), Error> {
        self.inner
            .send_bytes(payload.to_owned())
            .await
            .map_err(|e| Error::Internal(Box::new(e)))
    }
}

#[async_trait]
impl GracefulShutdown for SinkHalf<tide_websockets::WebSocketConnection, CannotSink> {
    async fn close(&mut self) {
        let close_msg = tide_websockets::Message::Close(None);
        self.inner
            .send(close_msg)
            .await
            .map_err(|err| Error::Internal(Box::new(err)))
            .unwrap_or_else(|err| log::error!("{}", err));
    }
}
