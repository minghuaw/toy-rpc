/// This modules implements `Server`'s methods that require `feature = "async_std_runtime"` 
/// or `feature = "http_tide"`.

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
        use ::async_std::net::{TcpListener, TcpStream};
        use ::async_std::task;
        use futures::StreamExt;
        use std::sync::Arc;
        use crate::codec::DefaultCodec;

        use crate::error::Error;
        use crate::transport::ws::WebSocketConn;

        use super::{AsyncServiceMap, Server};

        /// The following impl block is controlled by feature flag. It is enabled
        /// if and only if **exactly one** of the the following feature flag is turned on
        /// - `serde_bincode`
        /// - `serde_json`
        /// - `serde_cbor`
        /// - `serde_rmp`
        impl Server {
            /// Accepts connections on an `async_std::net::TcpListner` and serves requests to default
            /// server for each incoming connection.
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
            ///         .register("example", service!(example_service, ExampleService))
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
                let mut incoming = listener.incoming();

                while let Some(conn) = incoming.next().await {
                    let stream = conn?;

                    log::debug!("Accepting incoming connection from {}", stream.peer_addr()?);

                    task::spawn(Self::_serve_conn(stream, self.services.clone()));
                }

                Ok(())
            }

            /// Similar to `accept`. This will accept connections on an `async_std::net::TcpListner` and serves
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
            ///         .register("example", service!(example_service, ExampleService))
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
                let mut incoming = listener.incoming();

                while let Some(conn) = incoming.next().await {
                    let stream = conn?;
                    log::debug!("Accepting incoming connection from {}", stream.peer_addr()?);

                    let ws_stream = async_tungstenite::accept_async(stream).await?;

                    task::spawn(Self::_serve_websocket(ws_stream, self.services.clone()));
                }

                unimplemented!()
            }

            /// Serves a single connection
            async fn _serve_conn(stream: TcpStream, services: Arc<AsyncServiceMap>) -> Result<(), Error> {
                // let _stream = stream;
                let _peer_addr = stream.peer_addr()?;

                // using feature flag controlled default codec
                let codec = DefaultCodec::new(stream);

                // let fut = task::spawn_blocking(|| Self::_serve_codec(codec, services)).await;
                let ret = Self::_serve_codec(codec, services).await;
                log::debug!("Client disconnected from {}", _peer_addr);
                ret
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
            ///         .register("example", service!(example_service, ExampleService))
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
                Self::_serve_conn(stream, self.services.clone()).await
            }

            async fn _serve_websocket(
                ws_stream: async_tungstenite::WebSocketStream<TcpStream>,
                services: Arc<AsyncServiceMap>,
            ) -> Result<(), Error> {
                let ws_stream = WebSocketConn::new(ws_stream);
                let codec = DefaultCodec::with_websocket(ws_stream);

                let ret = Self::_serve_codec(codec, services).await;
                log::debug!("Client disconnected");
                ret
            }
        }


    }
}
