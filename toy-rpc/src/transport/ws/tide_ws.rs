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
    async fn read_payload(&mut self) -> Option<Result<Vec<u8>, IoError>> {
        match self.inner.next().await? {
            Err(e) => {
                return Some(Err(std::io::Error::new(
                    ErrorKind::InvalidData,
                    e.to_string(),
                )))
            }
            Ok(msg) => {
                if let tide_websockets::Message::Binary(bytes) = msg {
                    return Some(Ok(bytes));
                } else if let tide_websockets::Message::Close(_) = msg {
                    return None;
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
impl PayloadWrite for SinkHalf<tide_websockets::WebSocketConnection, CannotSink> {
    async fn write_payload(&mut self, payload: &[u8]) -> Result<(), IoError> {
        match self.inner.send_bytes(payload.to_owned()).await {
            Ok(_) => Ok(()),
            Err(err) => {
                match err {
                    tungstenite::error::Error::Io(e) => Err(e),
                    _ => Err(into_io_err_other(&err))
                }
            }
        }
    }
}

#[async_trait]
impl GracefulShutdown for SinkHalf<tide_websockets::WebSocketConnection, CannotSink> {
    async fn close(&mut self) {
        let msg = tide_websockets::Message::Close(None);

        if let Err(err) = self.inner.send(msg).await {
            match err {
                tungstenite::Error::ConnectionClosed => { },
                // tungstenite::Error::AlreadyClosed => { },
                e @ _ => {
                    log::error!("{}", e)
                }
            }
        }
    }
}
