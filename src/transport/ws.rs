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



// use async_std::net::TcpStream;
// use std::pin::Pin;
// use std::task::{Context, Poll};
use std::marker::PhantomData;
use futures::{AsyncRead, AsyncWrite, Sink, SinkExt, Stream, StreamExt, future::UnitError};
use pin_project::pin_project;
use async_trait::async_trait;
use tungstenite::Message as WsMessage;
use tungstenite::error::Error as WsError;

use crate::error::Error;

#[async_trait]
pub trait PayloadRead {
    async fn read_payload(&self) -> Vec<u8>;
}

#[async_trait]
pub trait PayloadWrite {
    async fn write_payload(&mut self, payload: &[u8]) -> Result<(), Error>;
}

/// type state for WebSocketConn
/// This is to separate `PayloadWrite` impl for `tide-websockets` 
/// which currently does not implement `Sink`
struct CanSink { }
struct CannotSink { }

#[pin_project]
pub struct WebSocketConn<T, N> {
    #[pin]
    pub inner: T,
    can_sink: PhantomData<N>,
}

// =============================================================================
// async-tungstenite,
// tokio-tungstenite,
// warp::filters::ws::WebSocket
// =============================================================================

impl<S> WebSocketConn<async_tungstenite::WebSocketStream<S>, CanSink> {
    pub fn new(stream: async_tungstenite::WebSocketStream<S>) -> Self {
        Self {
            inner: stream,
            can_sink: PhantomData
        }
    }
}

impl<S> WebSocketConn<tokio_tungstenite::WebSocketStream<S>, CanSink> {
    pub fn new(stream: tokio_tungstenite::WebSocketStream<S>) -> Self {
        Self {
            inner: stream,
            can_sink: PhantomData,
        }
    }
}

impl WebSocketConn<warp::filters::ws::WebSocket, CanSink> {
    pub fn new(stream: warp::filters::ws::WebSocket) -> Self {
        Self {
            inner: stream,
            can_sink: PhantomData,
        }
    }
}

// impl WebSocketConn<warp::filters::ws::WebSocket, CanStream>

#[async_trait]
impl<S, E, N> PayloadRead for WebSocketConn<S, N> 
where 
    S: Stream<Item = Result<WsMessage, E>> + Send + Sync + Unpin,
    E: std::error::Error + 'static,
    N: Send + Sync,
{
    async fn read_payload(&self) -> Vec<u8> {
        unimplemented!()
    }
}

#[async_trait]
impl<S> PayloadWrite for WebSocketConn<S, CanSink>
where 
    S: Sink<WsMessage> + Send + Sync + Unpin,
{
    async fn write_payload(&mut self, payload: &[u8]) -> Result<(), Error> {
        unimplemented!()
    }
}

// =============================================================================
// tide-websockets
// =============================================================================

impl WebSocketConn<tide_websockets::WebSocketConnection, CannotSink> {
    pub fn new(stream: tide_websockets::WebSocketConnection) -> Self {
        Self {
            inner: stream,
            can_sink: PhantomData,
        }
    }
}

#[async_trait]
impl PayloadWrite for WebSocketConn<tide_websockets::WebSocketConnection, CannotSink>{
    async fn write_payload(&mut self, payload: &[u8]) -> Result<(), Error> {
        unimplemented!()
    }
}

// =============================================================================
// actix-web-actors::ws
// This will have to implement on the `Server` type to handle websocket stream
// =============================================================================

#[cfg(test)]
mod tests {
    use async_std::net::{TcpListener, TcpStream};
    use async_std::task;

    #[test]
    fn async_tungestenite_handshake() {
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
        let url = url::Url::parse("ws://localhost:12345/").unwrap();
        let _stream = async_tungstenite::client_async(url, tcp)
            .await
            .expect("Client failed to connect");
    }
}