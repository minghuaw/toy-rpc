//! This modules implements `Server`'s methods that require `feature = "tokio_runtime"`,
//! `feature = "http_actix_web"` or `feature = "http_warp"`.


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
        use ::tokio::net::{TcpListener, TcpStream};
        use futures::{StreamExt};
        use ::tokio::task::{self};
        use tokio::io::{AsyncRead, AsyncWrite};
        use flume::Sender;
        use std::sync::atomic::Ordering;

        #[cfg(feature = "tls")]
        use tokio_rustls::{TlsAcceptor};
        #[cfg(feature = "tls")]
        use rustls::ServerConfig;

        use crate::error::Error;
        use crate::transport::ws::WebSocketConn;
        use crate::codec::split::SplittableCodec;
        use crate::codec::DefaultCodec;
        use super::{AsyncServiceMap, Server, ClientId, pubsub::PubSubItem};

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
            /// let example_service = Arc::new(ExampleService {});
            /// let server = Server::builder()
            ///     .register(example_service)
            ///     .build();
            /// let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
            /// server.accept(listener).await.unwrap();
            /// ```
            ///
            /// See `toy-rpc/examples/tokio_tcp/` for the example
            #[cfg_attr(feature = "docs", doc(cfg(feature = "tokio_runtime")))]
            pub async fn accept(&self, listener: TcpListener) -> Result<(), Error> {
                let mut incoming = tokio_stream::wrappers::TcpListenerStream::new(listener);

                while let Some(conn) = incoming.next().await {
                    let stream = conn?;
                    log::info!("Accepting incoming connection from {}", stream.peer_addr()?);

                    let client_id = self.client_counter.fetch_add(1, Ordering::Relaxed);
                    let pubsub_broker = self.pubsub_tx.clone();
                    task::spawn(
                        serve_tcp_connection(stream, self.services.clone(), client_id, pubsub_broker)
                    );
                }

                Ok(())
            }

            /// Accepts connections with TLS
            ///
            /// TLS is handled using `rustls`. A more detailed example with
            /// `tokio` runtime can be found in the [GitHub repo](https://github.com/minghuaw/toy-rpc/blob/9793bf53909bd7ffa74967fae6267f973e03ec8a/examples/tokio_tls/src/bin/server.rs#L43)
            #[cfg(feature = "tls")]
            #[cfg_attr(feature = "docs",doc(cfg(all(feature ="tls", feature = "tokio_runtime"))))]
            pub async fn accept_with_tls_config(&self, listener: TcpListener, config: ServerConfig) -> Result<(), Error> {
                let mut incoming = tokio_stream::wrappers::TcpListenerStream::new(listener);
                let acceptor = TlsAcceptor::from(Arc::new(config));

                while let Some(conn) = incoming.next().await {
                    let stream = conn?;
                    let acceptor = acceptor.clone();

                    let client_id = self.client_counter.fetch_add(1, Ordering::Relaxed);
                    let pubsub_broker = self.pubsub_tx.clone();
                    task::spawn(
                        serve_tls_connection(stream, acceptor, self.services.clone(), client_id, pubsub_broker)
                    );
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
            /// let example_service = Arc::new(ExampleService {});
            /// let server = Server::builder()
            ///     .register(example_service)
            ///     .build();
            /// let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
            /// server.accept_websocket(listener).await.unwrap();
            /// ```
            #[cfg_attr(feature = "docs", doc(cfg(feature = "tokio_runtime")))]
            pub async fn accept_websocket(&self, listener: TcpListener) -> Result<(), Error> {
                let mut incoming = tokio_stream::wrappers::TcpListenerStream::new(listener);

                while let Some(conn) = incoming.next().await {
                    let stream = conn?;
                    log::info!("Accepting incoming connection from {}", stream.peer_addr()?);

                    let client_id = self.client_counter.fetch_add(1, Ordering::Relaxed);
                    let pubsub_broker = self.pubsub_tx.clone();
                    task::spawn(
                        accept_ws_connection(stream, self.services.clone(), client_id, pubsub_broker)
                    );
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
            /// let conn = tokio::net::TcpStream::connect(addr).await.unwrap();
            /// server.serve_conn(conn).await.unwrap();
            /// ```
            // #[deprecated(
            //     since = "0.7.3",
            //     note = "Please use the serve_stream function instead"
            // )]
            // #[cfg_attr(feature = "docs", doc(cfg(feature = "tokio_runtime")))]
            // pub async fn serve_conn(&self, stream: TcpStream) -> Result<(), Error> {
            //     serve_tcp_connection(stream, self.services.clone()).await
            // }

            /// Serves a stream that implements `tokio::io::AsyncRead` and `tokio::io::AsyncWrite`
            #[cfg_attr(feature = "docs", doc(cfg(feature = "tokio_runtime")))]
            pub async fn serve_stream<T>(&self, stream: T) -> Result<(), Error> 
            where 
                T: AsyncRead + AsyncWrite + Send + Unpin + 'static
            {
                // let ret = serve_readwrite_stream(stream, self.services.clone()).await;
                let codec = DefaultCodec::new(stream);
                let ret = self.serve_codec(codec).await;
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
            #[cfg_attr(feature = "docs", doc(cfg(feature = "tokio_runtime")))]
            pub async fn serve_codec<C>(&self, codec: C) -> Result<(), Error>
            where
                C: SplittableCodec + Send + 'static,
            {
                let client_id = self.client_counter.fetch_add(1, Ordering::Relaxed);
                let pubsub_broker = self.pubsub_tx.clone();
                super::start_broker_reader_writer(codec, self.services.clone(), client_id, pubsub_broker).await
            }
        }

        #[cfg(feature = "tls")]
        async fn serve_tls_connection(
            stream: TcpStream,
            acceptor: TlsAcceptor,
            services: Arc<AsyncServiceMap>,
            client_id: ClientId, 
            pubsub_broker: Sender<PubSubItem>
        ) -> Result<(), Error> {
            let peer_addr = stream.peer_addr()?;
            let tls_stream = acceptor.accept(stream).await?;
            // let ret = serve_readwrite_stream(tls_stream, services).await;
            let codec = DefaultCodec::new(tls_stream);
            let ret = super::start_broker_reader_writer(codec, services, client_id, pubsub_broker).await;
            log::info!("Client disconnected from {}", peer_addr);
            ret
        }

        /// Serves a single connection
        async fn serve_tcp_connection(
            stream: TcpStream, 
            services: Arc<AsyncServiceMap>,
            client_id: ClientId, 
            pubsub_broker: Sender<PubSubItem>
        ) -> Result<(), Error> {
            let _peer_addr = stream.peer_addr()?;
            // let ret = serve_readwrite_stream(stream, services, client_id, pubsub_broker);
            let codec = DefaultCodec::new(stream);
            let ret = super::start_broker_reader_writer(codec, services, client_id, pubsub_broker).await;
            log::info!("Client disconnected from {}", _peer_addr);
            ret
        }

        async fn accept_ws_connection(
            stream: TcpStream, 
            services: Arc<AsyncServiceMap>,
            client_id: ClientId, 
            pubsub_broker: Sender<PubSubItem>
        ) {
            let ws_stream = async_tungstenite::tokio::accept_async(stream).await
                    .expect("Error during the websocket handshake occurred");
                log::debug!("Established WebSocket connection.");

            let ws_stream = WebSocketConn::new(ws_stream);
            let codec = DefaultCodec::with_websocket(ws_stream);

            if let Err(err) = super::start_broker_reader_writer(codec, services, client_id, pubsub_broker).await {
                log::error!("{}", err);
            }
            log::info!("Client disconnected from WebSocket connection");
        }
    }
}
