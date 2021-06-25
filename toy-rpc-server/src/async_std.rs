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
        use futures::io::{AsyncRead, AsyncWrite};

        #[cfg(feature = "tls")]
        use async_rustls::{TlsAcceptor};
        #[cfg(feature = "tls")]
        use rustls::ServerConfig;

        use toy_rpc_core::{
            error::Error,
            transport::ws::WebSocketConn,
            codec::{
                split::SplittableServerCodec, DefaultCodec
            },
        };

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
            /// let example_service = Arc::new(ExampleService {});
            /// let server = Server::builder()
            ///     .register(example_service)
            ///     .build();
            /// let listener = async_std::net::TcpListener::bind(addr).await.unwrap();
            /// server.accept(listener).await.unwrap();
            /// ```
            ///
            /// See `toy-rpc/examples/rap_tcp/` for the example
            #[cfg_attr(feature = "docs", doc(cfg(feature = "async_std_runtime")))]
            pub async fn accept(&self, listener: TcpListener) -> Result<(), Error> {
                let mut incoming = listener.incoming();

                while let Some(conn) = incoming.next().await {
                    let stream = conn?;
                    log::info!("Accepting incoming connection from {}", stream.peer_addr()?);

                    task::spawn(serve_tcp_connection(stream, self.services.clone()));
                }

                Ok(())
            }

            /// Accepts connections with TLS
            ///
            /// TLS is handled using `rustls`. A more detailed example with
            /// `tokio` runtime can be found in the [GitHub repo](https://github.com/minghuaw/toy-rpc/blob/9793bf53909bd7ffa74967fae6267f973e03ec8a/examples/async_std_tls/src/bin/server.rs#L43)
            #[cfg(feature = "tls")]
            #[cfg_attr(feature = "docs",doc(cfg(all(feature ="tls", feature = "async_std_runtime"))))]
            pub async fn accept_with_tls_config(&self, listener: TcpListener, config: ServerConfig) -> Result<(), Error> {
                let mut incoming = listener.incoming();
                let acceptor = TlsAcceptor::from(Arc::new(config));

                while let Some(conn) = incoming.next().await {
                    let stream = conn?;
                    let acceptor = acceptor.clone();

                    task::spawn(serve_tls_connection(stream, acceptor, self.services.clone()));
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
            /// let example_service = Arc::new(ExampleService {});
            /// let server = Server::builder()
            ///     .register(example_service)
            ///     .build();
            /// let listener = async_std::net::TcpListener::bind(addr).await.unwrap();
            /// server.accept_websocket(listener).await.unwrap();
            /// ```
            #[cfg_attr(feature = "docs", doc(cfg(feature = "async_std_runtime")))]
            pub async fn accept_websocket(&self, listener: TcpListener) -> Result<(), Error> {
                let mut incoming = listener.incoming();

                while let Some(conn) = incoming.next().await {
                    let stream = conn?;
                    log::info!("Accepting incoming connection from {}", stream.peer_addr()?);

                    task::spawn(accept_ws_connection(stream, self.services.clone()));
                }

                Ok(())
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
            /// let example_service = ExampleService {};
            /// let server = Server::builder()
            ///     .register(example_service)
            ///     .build();
            /// let conn = async_std::net::TcpStream::connect(addr).await.unwrap();
            /// server.serve_conn(conn).await.unwrap();
            /// ```
            #[deprecated(
                since = "0.7.3",
                note = "Please use the serve_stream function instead"
            )]
            #[cfg_attr(feature = "docs", doc(cfg(feature = "async_std_runtime")))]
            pub async fn serve_conn(&self, stream: TcpStream) -> Result<(), Error> {
                serve_tcp_connection(stream, self.services.clone()).await
            }

            /// Serves a stream that implements `futures::io::AsyncRead` and `futures::io::AsyncWrite`
            #[cfg_attr(feature = "docs", doc(cfg(feature = "async_std_runtime")))]
            pub async fn serve_stream<T>(&self, stream: T) -> Result<(), Error> 
            where 
                T: AsyncRead + AsyncWrite + Send + Unpin + 'static
            {
                let ret = serve_readwrite_stream(stream, self.services.clone()).await;
                log::info!("Client disconnected from stream");
                ret
            }

            /// This is like serve_conn except that it uses a specified codec
            ///
            /// Example
            ///
            /// ```rust
            /// let stream = TcpStream::connect("127.0.0.1:8080").await.unwrap();
            /// let codec = toy_rpc::codec::Codec::new(stream);
            /// let server = Server::builder()
            ///     .register(example_service)
            ///     .build();
            /// server.serve_codec(codec).await.unwrap();
            /// ```
            #[cfg_attr(feature = "docs", doc(cfg(feature = "async_std_runtime")))]
            pub async fn serve_codec<C>(&self, codec: C) -> Result<(), Error>
            where
                C: SplittableServerCodec + Send + 'static,
            {
                super::serve_codec_setup(codec, self.services.clone()).await
            }
        }

        #[cfg(feature = "tls")]
        async fn serve_tls_connection(
            stream: TcpStream,
            acceptor: TlsAcceptor,
            services: Arc<AsyncServiceMap>
        ) -> Result<(), Error> {
            let peer_addr = stream.peer_addr()?;
            let tls_stream = acceptor.accept(stream).await?;
            let ret = serve_readwrite_stream(tls_stream, services).await;
            log::info!("Client disconnected from {}", peer_addr);
            ret
        }

        /// Serves a single connection
        async fn serve_tcp_connection(stream: TcpStream, services: Arc<AsyncServiceMap>) -> Result<(), Error> {
            let _peer_addr = stream.peer_addr()?;
            let ret = serve_readwrite_stream(stream, services).await;
            log::info!("Client disconnected from {}", _peer_addr);
            ret
        }

        async fn accept_ws_connection(stream: TcpStream, services: Arc<AsyncServiceMap>) {
            let ws_stream = async_tungstenite::accept_async(stream).await
                    .expect("Error during the websocket handshake occurred");
                log::debug!("Established WebSocket connection.");

            serve_ws_connection(ws_stream, services).await
                .unwrap_or_else(|e| log::error!("{}", e));
        }

        async fn serve_ws_connection(
            ws_stream: async_tungstenite::WebSocketStream<TcpStream>,
            services: Arc<AsyncServiceMap>,
        ) -> Result<(), Error> {
            let ws_stream = WebSocketConn::new(ws_stream);
            let codec = DefaultCodec::with_websocket(ws_stream);

            let ret = super::serve_codec_setup(codec, services).await;
            log::info!("Client disconnected from WebSocket connection");
            ret
        }

        #[inline]
        async fn serve_readwrite_stream<T>(stream: T, services: Arc<AsyncServiceMap>) -> Result<(), Error> 
        where 
            T: AsyncRead + AsyncWrite + Send + Unpin + 'static,
        {
            let codec = DefaultCodec::new(stream);
            super::serve_codec_setup(codec, services).await
        }
    }
}