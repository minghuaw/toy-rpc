use tide_websockets as tide_ws;

use super::*;

// #[cfg(all(feature = "http_tide"))]
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

// #[cfg(all(feature = "http_tide"))]
#[async_trait]
impl PayloadRead for StreamHalf<tide_websockets::WebSocketConnection, CannotSink> {
    async fn read_payload(&mut self) -> Option<Result<Vec<u8>, Error>> {
        match self.inner.next().await? {
            Err(e) => return Some(Err(Error::TransportError { msg: e.to_string() })),
            Ok(msg) => {
                if let tide_websockets::Message::Binary(bytes) = msg {
                    return Some(Ok(bytes));
                } else if let tide_websockets::Message::Close(_) = msg {
                    return None;
                }

                Some(Err(Error::TransportError {
                    msg: "Expecting WebSocket::Message::Binary, but found something else"
                        .to_string(),
                }))
            }
        }
    }
}

// #[cfg(all(feature = "http_tide"))]
#[async_trait]
impl PayloadWrite for SinkHalf<tide_websockets::WebSocketConnection, CannotSink> {
    async fn write_payload(&mut self, payload: Vec<u8>) -> Result<(), Error> {
        self.inner
            .send_bytes(payload.into())
            .await
            .map_err(|e| Error::TransportError { msg: e.to_string() })
    }
}

// #[cfg(all(feature = "http_tide"))]
#[async_trait]
impl GracefulShutdown for SinkHalf<tide_websockets::WebSocketConnection, CannotSink> {
    async fn close(&mut self) {
        // tide-websocket does not provide a close method
        // let msg = WsMessage::Close(None);

        // self.inner
        //     .send(msg)
        //     .await
        //     .map_err(|e| Error::TransportError { msg: e.to_string() })
    }
}
