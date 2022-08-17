//! Client builder

use cfg_if::cfg_if;

use crate::pubsub::{DEFAULT_PUB_RETRIES, DEFAULT_PUB_RETRY_TIMEOUT};

#[cfg(any(feature = "ws_tokio", feature = "ws_async_std"))]
use crate::transport::ws::WebSocketConn;

cfg_if! {
    if #[cfg(any(
        feature = "docs",
        all(feature = "tokio_runtime", not(feature = "async_std_runtime"))
    ))] {
        #[cfg(feature = "tls")]
        use tokio_rustls::TlsConnector;
        #[cfg(all(feature = "tls", feature = "ws_tokio"))]
        use async_tungstenite::tokio::client_async;
        use tokio::net::TcpStream;

        use tokio::net::ToSocketAddrs;
        use ::tokio::io::{AsyncRead, AsyncWrite};

        #[cfg(any(feature = "ws_tokio", feature = "ws_async_std"))]
        use async_tungstenite::tokio::connect_async;
    } else if #[cfg(any(
        feature = "docs",
        all(feature = "async_std_runtime", not(feature = "tokio_runtime"))
    ))] {
        #[cfg(feature = "tls")]
        use futures_rustls::TlsConnector;
        #[cfg(all(feature = "tls", feature = "ws_async_std"))]
        use async_tungstenite::client_async;
        use async_std::net::TcpStream;

        use async_std::net::ToSocketAddrs;
        use futures::{AsyncRead, AsyncWrite};

        #[cfg(any(feature = "ws_tokio", feature = "ws_async_std"))]
        use async_tungstenite::async_std::connect_async;
    }
}

/// Client builder
pub struct ClientBuilder {
    // pub ack_mode: PhantomData<AckMode>,
    /// The duration a publisher waits for the Ack
    /// Waiting is non-blocking, and thus the publisher can still
    /// send out new Publish messages while waiting for the Ack of previous
    /// Publish message
    pub pub_retry_timeout: Duration,
    /// The number of retries that a publisher will attempt if Ack is not received.
    /// This only affects when Ack is enabled (ie. AckModeAuto, AckModeManual)
    pub max_num_retries: u32,
}

impl Default for ClientBuilder {
    fn default() -> Self {
        Self {
            // ack_mode: PhantomData,
            pub_retry_timeout: DEFAULT_PUB_RETRY_TIMEOUT,
            max_num_retries: DEFAULT_PUB_RETRIES,
        }
    }
}

impl ClientBuilder {
    /// Creates a new ClientBuilder
    pub fn new() -> ClientBuilder {
        ClientBuilder {
            // ack_mode: PhantomData,
            pub_retry_timeout: DEFAULT_PUB_RETRY_TIMEOUT,
            max_num_retries: DEFAULT_PUB_RETRIES,
        }
    }

    // /// Set the AckMode to None
    // pub fn set_ack_mode_none(self) -> ClientBuilder<AckModeNone> {
    //     ClientBuilder::<AckModeNone> {
    //         ack_mode: PhantomData,
    //         pub_retry_timeout: self.pub_retry_timeout,
    //         max_num_retries: self.max_num_retries,
    //     }
    // }

    // /// Set the AckMode to Auto
    // pub fn set_ack_mode_auto(self) -> ClientBuilder<AckModeAuto> {
    //     ClientBuilder::<AckModeAuto> {
    //         ack_mode: PhantomData,
    //         pub_retry_timeout: self.pub_retry_timeout,
    //         max_num_retries: self.max_num_retries,
    //     }
    // }

    // /// Set the AckMode to Manual
    // pub fn set_ack_mode_manual(self) -> ClientBuilder<AckModeManual> {
    //     ClientBuilder::<AckModeManual> {
    //         ack_mode: PhantomData,
    //         pub_retry_timeout: self.pub_retry_timeout,
    //         max_num_retries: self.max_num_retries,
    //     }
    // }
}

// impl ClientBuilder<AckModeAuto> {
//     /// Sets the duration that a publisher waits for the Ack from the server. If an Ack is not
//     /// received before the duration expires, the publisher will try to re-send the Publish message.
//     ///
//     /// This does not affect the timeout from the Server to all the `Subscriber`s.
//     pub fn set_publisher_retry_timeout(mut self, duration: Duration) -> Self {
//         self.pub_retry_timeout = duration;
//         self
//     }

//     /// Set the number of retries for the publisher
//     ///
//     /// This does not affect the max number of retries from the Server to all the
//     /// `Subscriber`s.
//     pub fn set_publisher_max_num_retries(self, val: u32) -> Self {
//         Self {
//             max_num_retries: val,
//             ..self
//         }
//     }
// }

// impl ClientBuilder<AckModeManual> {
//     /// Sets the duration that a publisher waits for the Ack from the server. If an Ack is not
//     /// received before the duration expires, the publisher will try to re-send the Publish message.
//     pub fn set_publisher_retry_timeout(mut self, duration: Duration) -> Self {
//         self.pub_retry_timeout = duration;
//         self
//     }

//     /// Set the number of retries
//     pub fn set_max_num_retries(self, val: u32) -> Self {
//         Self {
//             max_num_retries: val,
//             ..self
//         }
//     }
// }

cfg_if! {
    if #[cfg(any(
        feature = "docs",
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
    ))] {
        use std::{
            sync::Arc, collections::HashMap, time::Duration,
        };

        #[cfg(feature = "tls")]
        use rustls::ClientConfig;
        use crossbeam::atomic::AtomicCell;

        use crate::{
            client::Client,
            error::Error,
            codec::{split::SplittableCodec, DefaultCodec},
            message::AtomicMessageId,
        };

        #[cfg(any(feature = "ws_tokio", feature = "ws_async_std"))]
        use crate::DEFAULT_RPC_PATH;

        use super::{reader::ClientReader, writer::ClientWriter, broker};

        // macro_rules! impl_client_builder_for_ack_modes {
        //     ($($ack_mode:ty),*) => {
        //         $(
        //             impl ClientBuilder<$ack_mode> {
        //                 #[cfg(all(
        //                     feature = "tls",
        //                     any(
        //                         all(feature = "tokio_runtime", not(feature = "async_std_runtime")),
        //                         all(feature = "async_std_runtime", not(feature = "tokio_runtime"))
        //                     )
        //                 ))]
        //                 async fn tcp_client_with_tls_config(
        //                     self,
        //                     addr: impl ToSocketAddrs,
        //                     domain: &str,
        //                     config: rustls::ClientConfig
        //                 ) -> Result<Client<$ack_mode>, Error> {
        //                     use rustls::client::ServerName;
        //                     use std::convert::TryFrom;

        //                     let stream = TcpStream::connect(addr).await?;
        //                     let connector = TlsConnector::from(std::sync::Arc::new(config));
        //                     let domain = ServerName::try_from(domain)
        //                         .map_err(|_| Error::Internal(Box::new(webpki::InvalidDnsNameError)))?;
        //                     let tls_stream = connector.connect(domain, stream).await?;

        //                     Ok(
        //                         ClientBuilder::<$ack_mode>::new()
        //                             .with_stream(tls_stream)
        //                     )
        //                 }

        //                 #[cfg(all(
        //                     feature = "tls",
        //                     any(
        //                         feature = "ws_tokio",
        //                         feature = "ws_async_std",
        //                     ),
        //                     any(
        //                         all(feature = "tokio_runtime", not(feature = "async_std_runtime")),
        //                         all(feature = "async_std_runtime", not(feature = "tokio_runtime"))
        //                     )
        //                 ))]
        //                 async fn websocket_client_with_tls_config(
        //                     self,
        //                     url: url::Url,
        //                     domain: &str,
        //                     config: rustls::ClientConfig,
        //                 ) -> Result<Client<$ack_mode>, Error> {
        //                     let host = url.host_str()
        //                         .ok_or(Error::Internal("Invalid host address".into()))?;
        //                     let port = url.port_or_known_default()
        //                         .ok_or(Error::Internal("Invalid port".into()))?;
        //                     let addr = (host, port);
        //                     let stream = TcpStream::connect(addr).await?;
        //                     let connector = TlsConnector::from(std::sync::Arc::new(config));
        //                     // let domain = webpki::DNSNameRef::try_from_ascii_str(domain)?;
        //                     let domain = rustls::client::ServerName::try_from(domain)
        //                         .map_err(|_| Error::Internal(Box::new(webpki::InvalidDnsNameError)))?;
        //                     let tls_stream = connector.connect(domain, stream).await?;
        //                     let (ws_stream, _) = client_async(url, tls_stream).await?;
        //                     let ws_stream = WebSocketConn::new(ws_stream);
        //                     let codec = DefaultCodec::with_websocket(ws_stream);
        //                     Ok(
        //                         ClientBuilder::<$ack_mode>::new()
        //                             .with_codec(codec)
        //                     )
        //                 }

        //                 #[cfg(any(feature = "ws_tokio", feature = "ws_async_std"))]
        //                 async fn dial_websocket_url(self, url: url::Url) -> Result<Client<$ack_mode>, Error> {
        //                     let (ws_stream, _) = connect_async(&url).await?;
        //                     let ws_stream = WebSocketConn::new(ws_stream);
        //                     let codec = DefaultCodec::with_websocket(ws_stream);
        //                     Ok(self.with_codec(codec))
        //                 }

        //                 /// Connects to an RPC server over socket at the specified network address
        //                 pub async fn dial(self, addr: impl ToSocketAddrs) -> Result<Client<$ack_mode>, Error> {
        //                     let stream = TcpStream::connect(addr).await?;
        //                     Ok(
        //                         ClientBuilder::<$ack_mode>::new()
        //                             .with_stream(stream)
        //                     )
        //                 }

        //                 /// Connects to an RPC server with TLS enabled
        //                 #[cfg(feature = "tls")]
        //                 pub async fn dial_with_tls_config(
        //                     self,
        //                     addr: impl ToSocketAddrs,
        //                     domain: &str,
        //                     config: ClientConfig
        //                 ) -> Result<Client<$ack_mode>, Error> {
        //                     self.tcp_client_with_tls_config(addr, domain, config).await
        //                 }

        //                 /// Connects to an HTTP RPC server at the specified network address using WebSocket and the defatul codec.
        //                 #[cfg(any(feature = "ws_tokio", feature = "ws_async_std"))]
        //                 #[cfg_attr(feature = "docs", doc(cfg(any(feature = "ws_tokio", feature = "ws_async_std"))))]
        //                 pub async fn dial_http(self, addr: &str) -> Result<Client<$ack_mode>, Error> {
        //                     let mut url = url::Url::parse(addr)?.join(DEFAULT_RPC_PATH)?;
        //                     url.set_scheme("ws").expect("Failed to change scheme to ws");

        //                     ClientBuilder::<$ack_mode>::new()
        //                         .dial_websocket_url(url).await
        //                 }

        //                 /// Connects to an HTTP RPC server with TLS enabled
        //                 #[cfg(all(
        //                     feature = "tls",
        //                     any(
        //                         feature = "ws_tokio",
        //                         feature = "ws_async_std",
        //                     )
        //                 ))]
        //                 pub async fn dial_http_with_tls_config(
        //                     self,
        //                     addr: &str,
        //                     domain: &str,
        //                     config: ClientConfig,
        //                 ) -> Result<Client<$ack_mode>, Error> {
        //                     let mut url = url::Url::parse(addr)?.join(DEFAULT_RPC_PATH)?;
        //                     url.set_scheme("ws").expect("Failed to change scheme to ws");

        //                     self.websocket_client_with_tls_config(url, domain, config).await
        //                 }

        //                 /// Similar to `dial`, this connects to an WebSocket RPC server at the specified network address using the defatul codec
        //                 ///
        //                 /// The difference between `dial_websocket` and `dial_http` is that, `dial_websocket` does not
        //                 /// append `DEFAULT_RPC_PATH="_rpc"` to the end of the addr.
        //                 #[cfg(any(feature = "ws_tokio", feature = "ws_async_std"))]
        //                 #[cfg_attr(feature = "docs", doc(cfg(any(feature = "ws_tokio", feature = "ws_async_std"))))]
        //                 pub async fn dial_websocket(self, addr: &str) -> Result<Client<$ack_mode>, Error> {
        //                     let url = url::Url::parse(addr)?;
        //                     self.dial_websocket_url(url).await
        //                 }

        //                 /// Similar to `dial_websocket` but with TLS enabled
        //                 #[cfg(all(
        //                     feature = "tls",
        //                     any(
        //                         feature = "ws_tokio",
        //                         feature = "ws_async_std",
        //                     )
        //                 ))]
        //                 pub async fn dial_websocket_with_tls_config(
        //                     self,
        //                     addr: &str,
        //                     domain: &str,
        //                     config: ClientConfig,
        //                 ) -> Result<Client<$ack_mode>, Error> {
        //                     let url = url::Url::parse(addr)?;
        //                     self.websocket_client_with_tls_config(url, domain, config).await
        //                 }

        //                 /// Creates an RPC `Client` over a stream
        //                 ///
        //                 #[cfg_attr(feature = "docs", doc(cfg(feature = "tokio_runtime")))]
        //                 pub fn with_stream<T>(self, stream: T) -> Client<$ack_mode>
        //                 where
        //                     T: AsyncRead + AsyncWrite + Send + Unpin + 'static,
        //                 {
        //                     let codec = DefaultCodec::new(stream);
        //                     self.with_codec(codec)
        //                 }

        //                 /// Creates an RPC 'Client` over socket with a specified codec
        //                 #[cfg_attr(feature = "docs", doc(cfg(all(feature = "async_std_runtime", not(feature = "tokio_runtime")))))]
        //                 #[cfg_attr(feature = "docs", doc(cfg(all(feature = "tokio_runtime", not(feature = "async_std_runtime")))))]
        //                 pub fn with_codec<C>(self, codec: C) -> Client<$ack_mode>
        //                 where
        //                     C: SplittableCodec + Send + 'static,
        //                 {
        //                     let count = Arc::new(AtomicMessageId::new(0));
        //                     let (writer, reader) = codec.split();

        //                     let reader = ClientReader { reader };
        //                     let writer = ClientWriter { writer };
        //                     let broker = broker::ClientBroker::<$ack_mode, C>::new(
        //                         count.clone(), self.pub_retry_timeout, self.max_num_retries
        //                     );
        //                     let (handle, broker) = brw::spawn(broker, reader, writer);

        //                     Client {
        //                         count,
        //                         default_timeout: Duration::from_secs(super::DEFAULT_TIMEOUT_SECONDS),
        //                         next_timeout: AtomicCell::new(None),
        //                         broker,
        //                         broker_handle: Some(handle),
        //                         subscriptions: HashMap::new(),

        //                         ack_mode: PhantomData
        //                     }
        //                 }
        //             }
        //         )*
        //     };
        // }

        // impl_client_builder_for_ack_modes!(AckModeNone, AckModeAuto, AckModeManual);

        impl ClientBuilder {
            #[cfg(all(
                feature = "tls",
                any(
                    all(feature = "tokio_runtime", not(feature = "async_std_runtime")),
                    all(feature = "async_std_runtime", not(feature = "tokio_runtime"))
                )
            ))]
            async fn tcp_client_with_tls_config(
                self,
                addr: impl ToSocketAddrs,
                domain: &str,
                config: rustls::ClientConfig
            ) -> Result<Client, Error> {
                use rustls::client::ServerName;
                use std::convert::TryFrom;

                let stream = TcpStream::connect(addr).await?;
                let connector = TlsConnector::from(std::sync::Arc::new(config));
                let domain = ServerName::try_from(domain)
                    .map_err(|_| Error::Internal(Box::new(webpki::InvalidDnsNameError)))?;
                let tls_stream = connector.connect(domain, stream).await?;

                Ok(
                    ClientBuilder::new()
                        .with_stream(tls_stream)
                )
            }

            #[cfg(all(
                feature = "tls",
                any(
                    feature = "ws_tokio",
                    feature = "ws_async_std",
                ),
                any(
                    all(feature = "tokio_runtime", not(feature = "async_std_runtime")),
                    all(feature = "async_std_runtime", not(feature = "tokio_runtime"))
                )
            ))]
            async fn websocket_client_with_tls_config(
                self,
                url: url::Url,
                domain: &str,
                config: rustls::ClientConfig,
            ) -> Result<Client, Error> {
                let host = url.host_str()
                    .ok_or(Error::Internal("Invalid host address".into()))?;
                let port = url.port_or_known_default()
                    .ok_or(Error::Internal("Invalid port".into()))?;
                let addr = (host, port);
                let stream = TcpStream::connect(addr).await?;
                let connector = TlsConnector::from(std::sync::Arc::new(config));
                // let domain = webpki::DNSNameRef::try_from_ascii_str(domain)?;
                let domain = rustls::client::ServerName::try_from(domain)
                    .map_err(|_| Error::Internal(Box::new(webpki::InvalidDnsNameError)))?;
                let tls_stream = connector.connect(domain, stream).await?;
                let (ws_stream, _) = client_async(url, tls_stream).await?;
                let ws_stream = WebSocketConn::new(ws_stream);
                let codec = DefaultCodec::with_websocket(ws_stream);
                Ok(
                    ClientBuilder::new()
                        .with_codec(codec)
                )
            }

            #[cfg(any(feature = "ws_tokio", feature = "ws_async_std"))]
            async fn dial_websocket_url(self, url: url::Url) -> Result<Client, Error> {
                let (ws_stream, _) = connect_async(&url).await?;
                let ws_stream = WebSocketConn::new(ws_stream);
                let codec = DefaultCodec::with_websocket(ws_stream);
                Ok(self.with_codec(codec))
            }

            /// Connects to an RPC server over socket at the specified network address
            pub async fn dial(self, addr: impl ToSocketAddrs) -> Result<Client, Error> {
                let stream = TcpStream::connect(addr).await?;
                Ok(
                    ClientBuilder::new()
                        .with_stream(stream)
                )
            }

            /// Connects to an RPC server with TLS enabled
            #[cfg(feature = "tls")]
            pub async fn dial_with_tls_config(
                self,
                addr: impl ToSocketAddrs,
                domain: &str,
                config: ClientConfig
            ) -> Result<Client, Error> {
                self.tcp_client_with_tls_config(addr, domain, config).await
            }

            /// Connects to an HTTP RPC server at the specified network address using WebSocket and the defatul codec.
            #[cfg(any(feature = "ws_tokio", feature = "ws_async_std"))]
            #[cfg_attr(feature = "docs", doc(cfg(any(feature = "ws_tokio", feature = "ws_async_std"))))]
            pub async fn dial_http(self, addr: &str) -> Result<Client, Error> {
                let mut url = url::Url::parse(addr)?.join(DEFAULT_RPC_PATH)?;
                url.set_scheme("ws").expect("Failed to change scheme to ws");

                ClientBuilder::new()
                    .dial_websocket_url(url).await
            }

            /// Connects to an HTTP RPC server with TLS enabled
            #[cfg(all(
                feature = "tls",
                any(
                    feature = "ws_tokio",
                    feature = "ws_async_std",
                )
            ))]
            pub async fn dial_http_with_tls_config(
                self,
                addr: &str,
                domain: &str,
                config: ClientConfig,
            ) -> Result<Client, Error> {
                let mut url = url::Url::parse(addr)?.join(DEFAULT_RPC_PATH)?;
                url.set_scheme("ws").expect("Failed to change scheme to ws");

                self.websocket_client_with_tls_config(url, domain, config).await
            }

            /// Similar to `dial`, this connects to an WebSocket RPC server at the specified network address using the defatul codec
            ///
            /// The difference between `dial_websocket` and `dial_http` is that, `dial_websocket` does not
            /// append `DEFAULT_RPC_PATH="_rpc"` to the end of the addr.
            #[cfg(any(feature = "ws_tokio", feature = "ws_async_std"))]
            #[cfg_attr(feature = "docs", doc(cfg(any(feature = "ws_tokio", feature = "ws_async_std"))))]
            pub async fn dial_websocket(self, addr: &str) -> Result<Client, Error> {
                let url = url::Url::parse(addr)?;
                self.dial_websocket_url(url).await
            }

            /// Similar to `dial_websocket` but with TLS enabled
            #[cfg(all(
                feature = "tls",
                any(
                    feature = "ws_tokio",
                    feature = "ws_async_std",
                )
            ))]
            pub async fn dial_websocket_with_tls_config(
                self,
                addr: &str,
                domain: &str,
                config: ClientConfig,
            ) -> Result<Client, Error> {
                let url = url::Url::parse(addr)?;
                self.websocket_client_with_tls_config(url, domain, config).await
            }

            /// Creates an RPC `Client` over a stream
            ///
            #[cfg_attr(feature = "docs", doc(cfg(feature = "tokio_runtime")))]
            pub fn with_stream<T>(self, stream: T) -> Client
            where
                T: AsyncRead + AsyncWrite + Send + Unpin + 'static,
            {
                let codec = DefaultCodec::new(stream);
                self.with_codec(codec)
            }

            /// Creates an RPC 'Client` over socket with a specified codec
            #[cfg_attr(feature = "docs", doc(cfg(all(feature = "async_std_runtime", not(feature = "tokio_runtime")))))]
            #[cfg_attr(feature = "docs", doc(cfg(all(feature = "tokio_runtime", not(feature = "async_std_runtime")))))]
            pub fn with_codec<C>(self, codec: C) -> Client
            where
                C: SplittableCodec + Send + 'static,
            {
                let count = Arc::new(AtomicMessageId::new(0));
                let (writer, reader) = codec.split();

                let reader = ClientReader { reader };
                let writer = ClientWriter { writer };
                let broker = broker::ClientBroker::<C>::new(
                    count.clone(), self.pub_retry_timeout, self.max_num_retries
                );
                // let (handle, broker) = brw::spawn(broker, reader, writer);

                // Client {
                //     count,
                //     default_timeout: Duration::from_secs(super::DEFAULT_TIMEOUT_SECONDS),
                //     next_timeout: AtomicCell::new(None),
                //     broker,
                //     broker_handle: Some(handle),
                //     subscriptions: HashMap::new(),

                //     // ack_mode: PhantomData
                // }

                todo!()
            }
        }
    }
}
