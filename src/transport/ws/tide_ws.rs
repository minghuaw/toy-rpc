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
            Err(e) => return Some(Err(Error::TransportError( e.to_string() ))),
            Ok(msg) => {
                if let tide_websockets::Message::Binary(bytes) = msg {
                    return Some(Ok(bytes));
                } else if let tide_websockets::Message::Close(_) = msg {
                    return None;
                }

                Some(Err(Error::TransportError (
                    "Expecting WebSocket::Message::Binary, but found something else"
                        .to_string(),
                )))
            }
        }
    }
}

#[async_trait]
impl PayloadWrite for SinkHalf<tide_websockets::WebSocketConnection, CannotSink> {
    async fn write_payload(&mut self, payload: Vec<u8>) -> Result<(), Error> {
        self.inner
            .send_bytes(payload.into())
            .await
            .map_err(|e| Error::TransportError ( e.to_string() ))
    }
}
