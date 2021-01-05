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
// macros
// =============================================================================

// macro_rules! impl_cansink_new {
//     // this evil monstrosity matches <A, B: T, C: S+T>
//     ($ty:ident < $( $N:ident $(: $b0:ident $(+$b:ident)* )? ),* >) =>
//     {
//         impl< $( $N $(: $b0 $(+$b)* )? ),* >
//             $crate::Greet
//             for $ty< $( $N ),* >
//         {
//             pub fn new(stream: $ty <$N>) -> Self {
//                 Self {
//                     inner: stream,
//                     can_sink: PhantomData
//                 }
//             }
//         }
//     };
//     // match when no type parameters are present
//     ($ty:ident) => {
//         impl_greet!($ty<>);
//     };
// }

// impl_websocketconn_cansink_new!()

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
    fn new(inner: S) -> Self {
        Self {
            inner,
            can_sink: PhantomData
        }
    }
}

#[async_trait]
impl<S, E> PayloadRead for WebSocketConn<S, CanSink> 
where 
    S: Stream<Item = Result<WsMessage, E>> + Sink<WsMessage> + Send + Sync + Unpin,
    E: std::error::Error + 'static,
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
    fn new_without_sink(inner: tide_websockets::WebSocketConnection) -> Self {
        Self {
            inner,
            can_sink: PhantomData
        }
    }
}

#[async_trait]
impl PayloadRead for WebSocketConn<tide_websockets::WebSocketConnection, CannotSink> {
    async fn read_payload(&self) -> Vec<u8> {
        unimplemented!()
    }
}

#[async_trait]
impl PayloadWrite for WebSocketConn<tide_websockets::WebSocketConnection, CannotSink> {
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
    use super::*;

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

    #[test]
    fn new_websocketconn() {
        async_std::task::block_on(test_new_on_async_tungstenite());
        async_std::task::block_on(test_new_on_tokio_tungstenite());
        // async_std::task::block_on(test_new_on_tide_websockets());
    }

    async fn test_new_on_async_tungstenite() {
        let stream = async_std::io::Cursor::new(vec![1,2,3]);
        let ws_stream = async_tungstenite::WebSocketStream::from_raw_socket(
            stream, 
            tungstenite::protocol::Role::Server,
            None
        ).await;
        let _ = WebSocketConn::new(ws_stream);
    }

    async fn test_new_on_tokio_tungstenite() {
        let stream = std::io::Cursor::new(vec![1,2,3]);
        let ws_stream = tokio_tungstenite::WebSocketStream::from_raw_socket(
            stream, 
            tungstenite::protocol::Role::Server,
            None
        ).await;
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