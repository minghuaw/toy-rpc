//! RPC server. There is only one `Server` defined, but some methods have
//! different implementations depending on the runtime feature flag
//!

use cfg_if::cfg_if;
use std::{
    marker::PhantomData,
    sync::{atomic::AtomicU64, Arc},
};

use crate::{pubsub::AckModeNone, service::AsyncServiceMap};

#[cfg(any(feature = "docs", not(feature = "http_actix_web")))]
use crate::pubsub::AckModeAuto;

cfg_if! {
    if #[cfg(any(
        feature = "docs",
        all(feature = "async_std_runtime", not(feature = "tokio_runtime")),
        all(feature = "tokio_runtime", not(feature = "async_std_runtime")),
    ))] {
        use flume::Sender;
        mod integration;
        mod broker;
        mod reader;
        mod writer;

        pub mod pubsub;
        use pubsub::{PubSubBroker, PubSubItem};
    }
}

pub mod builder;
use builder::ServerBuilder;

pub(crate) type ClientId = u64;
pub(crate) type AtomicClientId = AtomicU64;

/// Client ID 0 is reserved for publisher and subscriber on the server side.
/// Remote client have their ID starting from `RESERVED_CLIENT_ID + 1`
pub const RESERVED_CLIENT_ID: ClientId = 0;

/// RPC Server
///
/// ```
/// const DEFAULT_RPC_PATH: &str = "_rpc_";
/// ```
#[derive(Clone)]
pub struct Server<AckMode> {
    services: Arc<AsyncServiceMap>,
    client_counter: Arc<AtomicClientId>, // monotomically increase counter

    #[cfg(any(
        feature = "docs",
        all(feature = "async_std_runtime", not(feature = "tokio_runtime")),
        all(feature = "tokio_runtime", not(feature = "async_std_runtime")),
    ))]
    pubsub_tx: Sender<PubSubItem>,

    ack_mode: PhantomData<AckMode>,
}

// Drop is implemented here because only **ONE** PubSub broker is available
// on one server
#[cfg(any(
    feature = "docs",
    all(feature = "async_std_runtime", not(feature = "tokio_runtime")),
    all(feature = "tokio_runtime", not(feature = "async_std_runtime")),
))]
impl<AckMode> Drop for Server<AckMode> {
    fn drop(&mut self) {
        if let Err(err) = self.pubsub_tx.send(PubSubItem::Stop) {
            log::error!("{}", err);
        }
    }
}

impl Server<AckModeNone> {
    /// Creates a `ServerBuilder`
    ///
    /// Example
    ///
    /// ```rust
    /// use toy_rpc::server::{ServerBuilder, Server};
    ///
    /// let builder: ServerBuilder = Server::builder();
    /// ```
    pub fn builder() -> ServerBuilder<AckModeNone> {
        ServerBuilder::default()
    }
}

cfg_if! {
    if #[cfg(any(
        feature = "docs",
        all(feature = "tokio_runtime", not(feature = "async_std_runtime"), not(feature = "http_actix_web"))
    ))] {
        #[cfg(feature = "tls")]
        use tokio_rustls::{TlsAcceptor};
        use tokio::net::{TcpListener, TcpStream};
        use tokio::task::{self};
        use tokio::io::{AsyncRead, AsyncWrite};

        #[cfg(feature = "ws")]
        use async_tungstenite::tokio::accept_async;
    } else if #[cfg(all(feature = "async_std_runtime", not(feature = "tokio_runtime")))] {
        #[cfg(feature = "tls")]
        use async_rustls::{TlsAcceptor};
        use async_std::net::{TcpListener, TcpStream};
        use async_std::task::{self};
        use futures::io::{AsyncRead, AsyncWrite};

        #[cfg(feature = "ws")]
        use async_tungstenite::accept_async;
    }
}

cfg_if! {
    if #[cfg(any(
        feature = "docs",
        all(
            not(feature = "http_actix_web"),
            any(
                all(
                    feature = "serde_bincode",
                    not(any(feature = "serde_json", feature = "serde_cbor", feature = "serde_rmp"))
                ),
                all(
                    feature = "serde_cbor",
                    not(any(feature = "serde_json", feature = "serde_bincode", feature = "serde_rmp")),
                ),
                all(
                    feature = "serde_json",
                    not(any(feature = "serde_bincode", feature = "serde_cbor", feature = "serde_rmp")),
                ),
                all(
                    feature = "serde_rmp",
                    not(any(feature = "serde_cbor", feature = "serde_json", feature = "serde_bincode")),
                )
            )
        )
    ))] {
        #[cfg(feature = "tls")]
        use rustls::ServerConfig;

        use futures::{StreamExt};
        use std::sync::atomic::Ordering;

        use crate::{error::Error, codec::{split::SplittableCodec, DefaultCodec}};

        #[cfg(feature = "ws")]
        use crate::{transport::ws::WebSocketConn};

        macro_rules! impl_server_for_ack_modes {
            ($($ack_mode:ty),*) => {
                $(
                    /// The following impl block is controlled by feature flag. It is enabled
                    /// if and only if **exactly one** of the the following feature flag is turned on
                    /// - `serde_bincode`
                    /// - `serde_json`
                    /// - `serde_cbor`
                    /// - `serde_rmp`
                    impl Server<$ack_mode> {
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
                        pub async fn accept(&self, listener: TcpListener) -> Result<(), Error> {
                            #[cfg(all(feature = "tokio_runtime", not(feature = "async_std_runtime")))]
                            let mut incoming = tokio_stream::wrappers::TcpListenerStream::new(listener);
                            #[cfg(all(feature = "async_std_runtime", not(feature = "tokio_runtime")))]
                            let mut incoming = listener.incoming();

                            while let Some(conn) = incoming.next().await {
                                let stream = conn?;
                                log::info!("Accepting incoming connection from {}", stream.peer_addr()?);

                                let client_id = self.client_counter.fetch_add(1, Ordering::Relaxed);
                                let pubsub_broker = self.pubsub_tx.clone();
                                task::spawn(
                                    Self::serve_tcp_connection(stream, self.services.clone(), client_id, pubsub_broker)
                                );
                            }

                            Ok(())
                        }

                        /// Accepts connections with TLS
                        ///
                        /// TLS is handled using `rustls`. A more detailed example with
                        /// `tokio` runtime can be found in the [GitHub repo](https://github.com/minghuaw/toy-rpc/blob/9793bf53909bd7ffa74967fae6267f973e03ec8a/examples/tokio_tls/src/bin/server.rs#L43)
                        #[cfg(feature = "tls")]
                        #[cfg_attr(feature = "docs",doc(cfg(all(feature ="tls"))))]
                        pub async fn accept_with_tls_config(&self, listener: TcpListener, config: ServerConfig) -> Result<(), Error> {
                            #[cfg(all(feature = "tokio_runtime", not(feature = "async_std_runtime")))]
                            let mut incoming = tokio_stream::wrappers::TcpListenerStream::new(listener);
                            #[cfg(all(feature = "async_std_runtime", not(feature = "tokio_runtime")))]
                            let mut incoming = listener.incoming();

                            let acceptor = TlsAcceptor::from(Arc::new(config));

                            while let Some(conn) = incoming.next().await {
                                let stream = conn?;
                                let acceptor = acceptor.clone();

                                let client_id = self.client_counter.fetch_add(1, Ordering::Relaxed);
                                let pubsub_broker = self.pubsub_tx.clone();
                                task::spawn(
                                    Self::serve_tls_connection(stream, acceptor, self.services.clone(), client_id, pubsub_broker)
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
                        #[cfg(feature = "ws")]
                        #[cfg_attr(feature = "docs", doc(cfg(feature = "ws")))]
                        pub async fn accept_websocket(&self, listener: TcpListener) -> Result<(), Error> {
                            #[cfg(all(feature = "tokio_runtime", not(feature = "async_std_runtime")))]
                            let mut incoming = tokio_stream::wrappers::TcpListenerStream::new(listener);
                            #[cfg(all(feature = "async_std_runtime", not(feature = "tokio_runtime")))]
                            let mut incoming = listener.incoming();

                            while let Some(conn) = incoming.next().await {
                                let stream = conn?;
                                log::info!("Accepting incoming connection from {}", stream.peer_addr()?);

                                let client_id = self.client_counter.fetch_add(1, Ordering::Relaxed);
                                let pubsub_broker = self.pubsub_tx.clone();
                                task::spawn(
                                    Self::accept_ws_connection(stream, self.services.clone(), client_id, pubsub_broker)
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
                        pub async fn serve_codec<C>(&self, codec: C) -> Result<(), Error>
                        where
                            C: SplittableCodec + Send + 'static,
                        {
                            let client_id = self.client_counter.fetch_add(1, Ordering::Relaxed);
                            let pubsub_broker = self.pubsub_tx.clone();
                            Self::start_broker_reader_writer(codec, self.services.clone(), client_id, pubsub_broker).await
                        }
                    }

                    impl Server<$ack_mode> {
                        pub(crate) async fn start_broker_reader_writer(
                            codec: impl crate::codec::split::SplittableCodec + 'static,
                            services: Arc<AsyncServiceMap>,
                            client_id: ClientId,
                            pubsub_tx: Sender<PubSubItem>,
                        ) -> Result<(), crate::Error> {
                            let (writer, reader) = codec.split();

                            let reader = reader::ServerReader::new(reader, services);
                            let writer = writer::ServerWriter::new(writer);
                            let broker = broker::ServerBroker::<$ack_mode>::new(client_id, pubsub_tx);

                            let (broker_handle, _) = brw::spawn(broker, reader, writer);
                            let _ = broker_handle.await;
                            Ok(())
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
                            let ret = Self::start_broker_reader_writer(codec, services, client_id, pubsub_broker).await;
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
                            let ret = Self::start_broker_reader_writer(codec, services, client_id, pubsub_broker).await;
                            log::info!("Client disconnected from {}", _peer_addr);
                            ret
                        }

                        #[cfg(feature = "ws")]
                        async fn accept_ws_connection(
                            stream: TcpStream,
                            services: Arc<AsyncServiceMap>,
                            client_id: ClientId,
                            pubsub_broker: Sender<PubSubItem>
                        ) {
                            let ws_stream = accept_async(stream).await
                                    .expect("Error during the websocket handshake occurred");
                                log::debug!("Established WebSocket connection.");

                            let ws_stream = WebSocketConn::new(ws_stream);
                            let codec = DefaultCodec::with_websocket(ws_stream);

                            if let Err(err) = Self::start_broker_reader_writer(codec, services, client_id, pubsub_broker).await {
                                log::error!("{}", err);
                            }
                            log::info!("Client disconnected from WebSocket connection");
                        }
                    }
                )*
            }
        }

        impl_server_for_ack_modes!(AckModeNone);

        #[cfg(not(feature = "docs"))]
        impl_server_for_ack_modes!(AckModeAuto);
    }
}
