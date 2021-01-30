use async_trait::async_trait;
use futures::stream::{SplitSink, SplitStream};
use futures::{Sink, SinkExt, Stream, StreamExt};

use std::marker::PhantomData;
use tide_websockets as tide_ws;
use tungstenite::Message as WsMessage;

use crate::error::Error;
use super::{PayloadRead, PayloadWrite};

/// Websocket transport
///
/// WebSocket message definition:
///
/// - `tungstenite::Mesage` based: `tide-websocket` (wrapping `async-tungstenite`), `warp` (using `tokio-tungstenite`)
/// - `actix-web-actors::ws`: this seems like `actix-web`'s implementation.
///
/// `Stream` and `Sink<tungstenite::Message>`?
///
/// - `async-tungstenite` and `tokio-tungstenite` both impl `Stream` and `Sink<Message>`
/// - `warp::filters::ws::WebSocket` is a wrapper of `tokio::tungstenite::WebSocketStream`, and impls both `Stream` and `Sink<Message>`
/// - `tide-websocket` only impls `Stream` but not `Sink<Message>`
/// 
/// WebSocket connection process



/// type state for WebSocketConn
/// This is to separate `PayloadWrite` impl for `tide-websockets`
/// which currently does not implement `Sink`
pub(crate) struct CanSink {}
pub(crate) struct CannotSink {}

// #[pin_project]
pub struct WebSocketConn<S, N> {
    // #[pin]
    pub inner: S,
    can_sink: PhantomData<N>,
}

pub struct StreamHalf<S> {
    inner: S,
}

pub struct SinkHalf<S, Mode> {
    inner: S,
    can_sink: PhantomData<Mode>,
}

// =============================================================================
// async-tungstenite,
// tokio-tungstenite,
// warp::filters::ws::WebSocket
// =============================================================================

impl<S, E> WebSocketConn<S, CanSink>
where
    S: Stream<Item = Result<WsMessage, E>> + Sink<WsMessage> + Send + Sync + Unpin,
    E: std::error::Error + 'static,
{
    pub fn new(inner: S) -> Self {
        Self {
            inner,
            can_sink: PhantomData,
        }
    }

    pub fn split(
        self,
    ) -> (
        SinkHalf<SplitSink<S, WsMessage>, CanSink>,
        StreamHalf<SplitStream<S>>,
    ) {
        let (writer, reader) = self.inner.split();

        let readhalf = StreamHalf { inner: reader };
        let writehalf = SinkHalf {
            inner: writer,
            can_sink: PhantomData,
        };
        (writehalf, readhalf)
    }
}

#[async_trait]
impl<S, E> PayloadRead for StreamHalf<S>
where
    S: Stream<Item = Result<WsMessage, E>> + Send + Sync + Unpin,
    E: std::error::Error + 'static,
{
    async fn read_payload(&mut self) -> Option<Result<Vec<u8>, Error>> {
        match self.inner.next().await? {
            Err(e) => return Some(Err(Error::TransportError { msg: e.to_string() })),
            Ok(msg) => {
                if let WsMessage::Binary(bytes) = msg {
                    return Some(Ok(bytes));
                } else if let WsMessage::Close(_) = msg {
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

#[async_trait]
impl<S, E> PayloadWrite for SinkHalf<S, CanSink>
where
    S: Sink<WsMessage, Error = E> + Send + Sync + Unpin,
    E: std::error::Error + 'static,
{
    async fn write_payload(&mut self, payload: Vec<u8>) -> Result<(), Error> {
        let msg = WsMessage::Binary(payload.into());

        self.inner
            .send(msg)
            .await
            .map_err(|e| Error::TransportError { msg: e.to_string() })
    }
}

// =============================================================================
// tide-websockets
// =============================================================================

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
        StreamHalf<tide_ws::WebSocketConnection>,
    ) {
        let writer = SinkHalf {
            inner: self.inner.clone(),
            can_sink: PhantomData,
        };
        let reader = StreamHalf { inner: self.inner };
        (writer, reader)
    }
}

#[async_trait]
impl PayloadWrite for SinkHalf<tide_websockets::WebSocketConnection, CannotSink> {
    async fn write_payload(&mut self, payload: Vec<u8>) -> Result<(), Error> {
        self.inner
            .send_bytes(payload.into())
            .await
            .map_err(|e| Error::TransportError { msg: e.to_string() })
    }
}

// =============================================================================
// actix-web-actors::ws
// This will have to implement on the `Server` type to handle websocket stream
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use async_std::net::{TcpListener, TcpStream};
    use async_std::task;

    #[test]
    fn async_tungstenite_handshake() {
        println!("Start testing");
        task::block_on(test_handshake());
        println!("end testing");
    }

    async fn test_handshake() {
        let (tx, rx) = futures::channel::oneshot::channel();

        let f = async move {
            let listener = TcpListener::bind("0.0.0.0:12345").await.unwrap();
            tx.send(()).unwrap();
            while let Ok((connection, _)) = listener.accept().await {
                let stream = async_tungstenite::accept_async(connection).await;
                stream.expect("Failed to handshake with connection");
            }
        };

        task::spawn(f);

        rx.await.expect("Failed to wait for server to be ready");
        let tcp = TcpStream::connect("0.0.0.0:12345")
            .await
            .expect("Failed to connect");
        let mut url = url::Url::parse("http://localhost:12345/").unwrap();
        url.set_scheme("ws").unwrap();
        let _stream = async_tungstenite::client_async(url, tcp)
            .await
            .expect("Client failed to connect");
    }

    #[test]
    fn new_websocketconn() {
        async_std::task::block_on(test_new_on_async_tungstenite());
        async_std::task::block_on(test_new_on_tokio_tungstenite());
        // async_std::task::block_on(test_new_on_tide_websockets());
    }

    async fn test_new_on_async_tungstenite() {
        let stream = async_std::io::Cursor::new(vec![1, 2, 3]);
        let ws_stream = async_tungstenite::WebSocketStream::from_raw_socket(
            stream,
            tungstenite::protocol::Role::Server,
            None,
        )
        .await;
        let conn = WebSocketConn::new(ws_stream);

        let (_writer, _reader) = conn.split();
    }

    async fn test_new_on_tokio_tungstenite() {
        let stream = std::io::Cursor::new(vec![1, 2, 3]);
        let ws_stream = tokio_tungstenite::WebSocketStream::from_raw_socket(
            stream,
            tungstenite::protocol::Role::Server,
            None,
        )
        .await;
        let _ = WebSocketConn::new(ws_stream);
    }

    // async fn test_new_on_tide_websockets() {
    //     let stream = async_std::io::Cursor::new(vec![1,2,3]);
    //     let ws_stream = async_tungstenite::WebSocketStream::from_raw_socket(
    //         stream,
    //         tungstenite::protocol::Role::Server,
    //         None
    //     ).await;
    //     let tide_ws = tide_websockets::WebSocketConnection::from(ws_stream);
    //     let _ = WebSocketConn::new_with_sink(ws_stream);
    // }
}
