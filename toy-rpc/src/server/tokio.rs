/// This modules implements `Server`'s methods that require `feature = "tokio_runtime"`,
/// `feature = "http_actix_web"` or `feature = "http_warp"`.
use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(any(
        any(feature = "docs", doc),
        all(
            feature = "serde_bincode",
            not(feature = "serde_json"),
            not(feature = "serde_cbor"),
            not(feature = "serde_rmp"),
        ),
        all(
            feature = "serde_cbor",
            not(feature = "serde_json"),
            not(feature = "serde_bincode"),
            not(feature = "serde_rmp"),
        ),
        all(
            feature = "serde_json",
            not(feature = "serde_bincode"),
            not(feature = "serde_cbor"),
            not(feature = "serde_rmp"),
        ),
        all(
            feature = "serde_rmp",
            not(feature = "serde_cbor"),
            not(feature = "serde_json"),
            not(feature = "serde_bincode"),
        ),
    ))] {
        use ::tokio::net::{TcpListener, TcpStream};
        use futures::StreamExt;
        use ::tokio::task;
        use async_tungstenite::tokio::TokioAdapter;
        use std::sync::Arc;

        use crate::error::Error;
        use crate::transport::ws::WebSocketConn;
        use crate::codec::DefaultCodec;

        use super::{AsyncServiceMap, Server};

        /// The following impl block is controlled by feature flag. It is enabled
        /// if and only if **exactly one** of the the following feature flag is turned on
        /// - `serde_bincode`
        /// - `serde_json`
        /// - `serde_cbor`
        /// - `serde_rmp`
        impl Server {

            /// Accepts connections on an `tokio::net::TcpListener` and serves requests to default
            /// server for each incoming connection
            ///
            /// This is enabled
            /// if and only if **exactly one** of the the following feature flag is turned on
            /// - `serde_bincode`
            /// - `serde_json`
            /// - `serde_cbor`
            /// - `serde_rmp`
            ///
            /// # Example
            ///
            /// ```rust
            /// use async_std::net::TcpListener;
            ///
            /// async fn main() {
            ///     // assume `ExampleService` exist
            ///     let example_service = ExampleService {};
            ///     let server = Server::builder()
            ///         .register(example_service)
            ///         .build();
            ///
            ///     let listener = TcpListener::bind(addr).await.unwrap();
            ///
            ///     let handle = task::spawn(async move {
            ///         server.accept(listener).await.unwrap();
            ///     });
            ///     handle.await;
            /// }
            /// ```
            ///
            /// See `toy-rpc/examples/rap_tcp/` for the example
            pub async fn accept(&self, listener: TcpListener) -> Result<(), Error> {
                let mut incoming = tokio_stream::wrappers::TcpListenerStream::new(listener);

                while let Some(conn) = incoming.next().await {
                    let stream = conn?;

                    log::trace!("Accepting incoming connection from {}", stream.peer_addr()?);

                    task::spawn(Self::serve_tcp_connection(stream, self.services.clone()));
                }

                Ok(())
            }

            /// Similar to `accept`. This will accept connections on a `tokio::net::TcpListener` and serves
            /// requests using WebSocket transport protocol and the default codec.
            ///
            /// This is enabled
            /// if and only if **exactly one** of the the following feature flag is turned on
            /// - `serde_bincode`
            /// - `serde_json`
            /// - `serde_cbor`
            /// - `serde_rmp`
            ///
            /// # Example
            ///
            /// ```rust
            /// use async_std::net::TcpListener;
            ///
            /// async fn main() {
            ///     // assume `ExampleService` exist
            ///     let example_service = ExampleService {};
            ///     let server = Server::builder()
            ///         .register(example_service)
            ///         .build();
            ///
            ///     let listener = TcpListener::bind(addr).await.unwrap();
            ///
            ///     let handle = task::spawn(async move {
            ///         server.accept_websocket(listener).await.unwrap();
            ///     });
            ///     handle.await;
            /// }
            /// ```
            pub async fn accept_websocket(&self, listener: TcpListener) -> Result<(), Error> {
                let mut incoming = tokio_stream::wrappers::TcpListenerStream::new(listener);

                while let Some(conn) = incoming.next().await {
                    let stream = conn?;
                    log::trace!("Accepting incoming connection from {}", stream.peer_addr()?);

                    task::spawn(Self::accept_ws_connection(stream, self.services.clone()));
                }

                Ok(())
            }

            async fn accept_ws_connection(stream: TcpStream, services: Arc<AsyncServiceMap>) {
                let ws_stream = async_tungstenite::tokio::accept_async(stream).await
                        .expect("Error during the websocket handshake occurred");
                    log::trace!("Established WebSocket connection.");

                match Self::serve_ws_connection(ws_stream, services).await {
                    Ok(_) => {},
                    Err(e) => log::error!("{}", e),
                };
            }

            /// Serves a single connection
            async fn serve_tcp_connection(stream: TcpStream, services: Arc<AsyncServiceMap>) -> Result<(), Error> {
                // let _stream = stream;
                let _peer_addr = stream.peer_addr()?;

                // using feature flag controlled default codec
                let codec = DefaultCodec::new(stream);

                // let fut = task::spawn_blocking(|| Self::_serve_codec(codec, services)).await;
                let fut = Self::serve_codec_loop(codec, services);

                log::trace!("Client disconnected from {}", _peer_addr);

                fut.await
            }

            /// Serves a single connection using the default codec
            ///
            /// This is enabled
            /// if and only if **exactly one** of the the following feature flag is turned on
            /// - `serde_bincode`
            /// - `serde_json`
            /// - `serde_cbor`
            /// - `serde_rmp`
            ///
            /// Example
            ///
            /// ```rust
            /// use async_std::net::TcpStream;
            ///
            /// async fn main() {
            ///     // assume `ExampleService` exist
            ///     let example_service = ExampleService {};
            ///     let server = Server::builder()
            ///         .register(example_service)
            ///         .build();
            ///
            ///     let conn = TcpStream::connect(addr).await.unwrap();
            ///
            ///     let handle = task::spawn(async move {
            ///         server.serve_conn(conn).await.unwrap();
            ///     });
            ///     handle.await;
            /// }
            /// ```
            pub async fn serve_conn(&self, stream: TcpStream) -> Result<(), Error> {
                Self::serve_tcp_connection(stream, self.services.clone()).await
            }

            async fn serve_ws_connection(
                ws_stream: async_tungstenite::WebSocketStream<TokioAdapter<tokio::net::TcpStream>>,
                services: Arc<AsyncServiceMap>,
            ) -> Result<(), Error> {
                let ws_stream = WebSocketConn::new(ws_stream);
                let codec = DefaultCodec::with_websocket(ws_stream);

                let fut = Self::serve_codec_loop(codec, services);

                log::trace!("Client disconnected");

                fut.await
            }
        }

    }
}
