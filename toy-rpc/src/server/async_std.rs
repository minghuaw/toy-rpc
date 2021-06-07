//! This modules implements `Server`'s methods that require `feature = "async_std_runtime"`
//! or `feature = "http_tide"`.

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
        use std::sync::Arc;
        use ::async_std::net::{TcpListener, TcpStream};
        use ::async_std::task::{self};
        use futures::{StreamExt};

        use crate::error::Error;
        use crate::transport::ws::WebSocketConn;
        use crate::codec::split::SplittableServerCodec;
        use crate::codec::DefaultCodec;

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
                let mut incoming = listener.incoming();

                while let Some(conn) = incoming.next().await {
                    let stream = conn?;
                    log::info!("Accepting incoming connection from {}", stream.peer_addr()?);

                    task::spawn(Self::serve_tcp_connection(stream, self.services.clone()));
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
                let mut incoming = listener.incoming();

                while let Some(conn) = incoming.next().await {
                    let stream = conn?;
                    log::info!("Accepting incoming connection from {}", stream.peer_addr()?);

                    task::spawn(Self::accept_ws_connection(stream, self.services.clone()));
                }

                Ok(())
            }

            async fn accept_ws_connection(stream: TcpStream, services: Arc<AsyncServiceMap>) {
                let ws_stream = async_tungstenite::accept_async(stream).await
                        .expect("Error during the websocket handshake occurred");
                    log::debug!("Established WebSocket connection.");

                Self::serve_ws_connection(ws_stream, services).await 
                    .unwrap_or_else(|e| log::error!("{}", e));
            }

            /// Serves a single connection
            async fn serve_tcp_connection(stream: TcpStream, services: Arc<AsyncServiceMap>) -> Result<(), Error> {
                // let _stream = stream;
                let _peer_addr = stream.peer_addr()?;

                // using feature flag controlled default codec
                let codec = DefaultCodec::new(stream);

                // let fut = task::spawn_blocking(|| Self::_serve_codec(codec, services)).await;
                // let ret = Self::serve_codec_loop(codec, services).await;
                let ret = Self::serve_codec_setup(codec, services).await;
                log::info!("Client disconnected from {}", _peer_addr);
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
                ws_stream: async_tungstenite::WebSocketStream<TcpStream>,
                services: Arc<AsyncServiceMap>,
            ) -> Result<(), Error> {
                let ws_stream = WebSocketConn::new(ws_stream);
                let codec = DefaultCodec::with_websocket(ws_stream);

                let ret = Self::serve_codec_setup(codec, services).await;
                log::info!("Client disconnected from WebSocket connection");
                ret
            }

            /// This is like serve_conn except that it uses a specified codec
            ///
            /// Example
            ///
            /// ```rust
            /// let stream = TcpStream::connect("127.0.0.1:8080").await.unwrap();
            /// let codec = Codec::new(stream);
            /// 
            /// let server = Server::builder()
            ///     .register(example_service)
            ///     .build();
            /// // assume `ExampleService` exist
            /// let handle = task::spawn(async move {
            ///     server.serve_codec(codec).await.unwrap();
            /// })    
            /// handle.await;
            /// ```
            pub async fn serve_codec<C>(&self, codec: C) -> Result<(), Error>
            where
                C: SplittableServerCodec + Send + Sync + 'static,
            {
                Self::serve_codec_setup(codec, self.services.clone()).await
            }

            // Spawn tasks for the reader/broker/writer loops
            pub(crate) async fn serve_codec_setup(
                codec: impl SplittableServerCodec + 'static,
                services: Arc<AsyncServiceMap>
            ) -> Result<(), Error> {
                let (exec_sender, exec_recver) = flume::unbounded();
                let (resp_sender, resp_recver) = flume::unbounded();
                let (codec_writer, codec_reader) = codec.split();

                let reader_handle = task::spawn(
                    super::reader_loop(codec_reader, services, exec_sender.clone(), resp_sender.clone())
                );

                let writer_handle = task::spawn(
                    super::writer_loop(codec_writer, resp_recver)
                );

                let executor_handle = task::spawn(
                    super::broker_loop(exec_sender, exec_recver, resp_sender)
                );

                reader_handle.await?;
                executor_handle.await?;
                writer_handle.await?;
                Ok(())
            }
        }
    }
}
