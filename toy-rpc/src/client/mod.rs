//! RPC Client impementation

use cfg_if::cfg_if;
use crossbeam::atomic::AtomicCell;
use flume::Sender;
use std::{any::TypeId, collections::HashMap, marker::PhantomData, sync::Arc, time::Duration};

use crate::{message::AtomicMessageId, protocol::InboundBody, pubsub::AckModeNone};

pub(crate) mod broker;
pub mod builder;
pub mod pubsub;
mod reader;
mod writer;

use broker::ClientBrokerItem;

type ResponseResult = Result<Box<InboundBody>, Box<InboundBody>>;

cfg_if! {
    if #[cfg(any(
        feature = "docs",
        all(feature = "tokio_runtime", not(feature = "async_std_runtime")),
        all(feature = "async_std_runtime", not(feature = "tokio_runtime"))
    ))] {
        use futures::channel::oneshot;

        use crate::{Error, protocol::{OutboundBody}};
        use crate::transport::ws::WebSocketConn;
        use crate::codec::DefaultCodec;

        const DEFAULT_TIMEOUT_SECONDS: u64 = 10;
    }
}

cfg_if! {
    if #[cfg(all(feature = "tokio_runtime", not(feature = "async_std_runtime")))] {
        #[cfg(feature = "tls")]
        use tokio_rustls::TlsConnector;
        #[cfg(feature = "tls")]
        use async_tungstenite::tokio::client_async;

        use tokio::net::{TcpStream, ToSocketAddrs};
        use async_tungstenite::tokio::connect_async;
        use ::tokio::io::{AsyncRead, AsyncWrite};
    } else if #[cfg(all(feature = "async_std_runtime", not(feature = "tokio_runtime")))] {
        #[cfg(feature = "tls")]
        use async_rustls::TlsConnector;
        #[cfg(feature = "tls")]
        use async_tungstenite::client_async;

        use async_std::net::{TcpStream, ToSocketAddrs};
        use async_tungstenite::async_std::connect_async;
        use futures::{AsyncRead, AsyncWrite};
    }
}

cfg_if! {
    if #[cfg(any(
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
        #[cfg(feature = "tls")]
        use rustls::{ClientConfig};

        use crate::DEFAULT_RPC_PATH;

        #[cfg(all(
            feature = "tls",
            any(
                all(feature = "tokio_runtime", not(feature = "async_std_runtime")),
                all(feature = "async_std_runtime", not(feature = "tokio_runtime"))
            )
        ))]
        async fn tcp_client_with_tls_config<AckMode>(
            addr: impl ToSocketAddrs,
            domain: &str,
            config: rustls::ClientConfig
        ) -> Result<Client<AckMode>, Error> {
            let stream = TcpStream::connect(addr).await?;
            let connector = TlsConnector::from(std::sync::Arc::new(config));
            let domain = webpki::DNSNameRef::try_from_ascii_str(domain)?;
            let tls_stream = connector.connect(domain, stream).await?;

            Ok(Client::with_stream(tls_stream))
        }

        #[cfg(all(
            feature = "tls",
            any(
                all(feature = "tokio_runtime", not(feature = "async_std_runtime")),
                all(feature = "async_std_runtime", not(feature = "tokio_runtime"))
            )
        ))]
        async fn websocket_client_with_tls_config<AckMode>(
            url: url::Url,
            domain: &str,
            config: rustls::ClientConfig,
        ) -> Result<Client<AckMode>, Error> {
            let host = url.host_str()
                .ok_or(Error::Internal("Invalid host address".into()))?;
            let port = url.port_or_known_default()
                .ok_or(Error::Internal("Invalid port".into()))?;
            let addr = (host, port);
            let stream = TcpStream::connect(addr).await?;
            let connector = TlsConnector::from(std::sync::Arc::new(config));
            let domain = webpki::DNSNameRef::try_from_ascii_str(domain)?;
            let tls_stream = connector.connect(domain, stream).await?;
            let (ws_stream, _) = client_async(url, tls_stream).await?;
            let ws_stream = WebSocketConn::new(ws_stream);
            let codec = DefaultCodec::with_websocket(ws_stream);
            Ok(Client::with_codec(codec))
        }

        impl<AckMode> Client<AckMode> {
            pub(crate) async fn dial_websocket_url(url: url::Url) -> Result<Self, Error> {
                let (ws_stream, _) = connect_async(&url).await?;
                let ws_stream = WebSocketConn::new(ws_stream);
                let codec = DefaultCodec::with_websocket(ws_stream);
                Ok(Self::with_codec(codec))
            }

            /// Creates an RPC `Client` over a stream that implements `tokio::io::AsyncRead`
            /// and `tokio::io::AsyncWrite`
            ///
            /// This is enabled
            /// if and only if **exactly one** of the the following feature flag is turned on
            /// - `serde_bincode`
            /// - `serde_json`
            /// - `serde_cbor`
            /// - `serde_rmp`
            ///
            /// # Example
            /// ```
            /// let stream = TcpStream::connect("127.0.0.1:8080").await.unwrap();
            /// let client = Client::with_stream(stream);
            /// ```
            #[cfg_attr(feature = "docs", doc(cfg(feature = "tokio_runtime")))]
            pub fn with_stream<T>(stream: T) -> Self
            where
                T: AsyncRead + AsyncWrite + Send + Unpin + 'static,
            {
                let codec = DefaultCodec::new(stream);
                Self::with_codec(codec)
            }
        }

        /// The following impl block is controlled by feature flag. It is enabled
        /// if and only if **exactly one** of the the following feature flag is turned on
        /// - `serde_bincode`
        /// - `serde_json`
        /// - `serde_cbor`
        /// - `serde_rmp`
        impl Client<AckModeNone> {
            /// Connects to an RPC server over socket at the specified network address
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
            /// let addr = "127.0.0.1:8080";
            /// let client = Client::dial(addr).await.unwrap();
            /// ```
            #[cfg_attr(feature = "docs", doc(cfg(feature = "tokio_runtime")))]
            pub async fn dial(addr: impl ToSocketAddrs)
                -> Result<Self, Error>
            {
                let stream = TcpStream::connect(addr).await?;
                Ok(Self::with_stream(stream))
            }

            /// Connects to an RPC server with TLS enabled
            ///
            /// A more detailed example can be found in the
            /// [GitHub repo](https://github.com/minghuaw/toy-rpc/blob/9793bf53909bd7ffa74967fae6267f973e03ec8a/examples/tokio_tls/src/bin/client.rs#L22)
            #[cfg(feature = "tls")]
            #[cfg_attr(feature = "docs",doc(cfg(all(feature ="tls", feature = "tokio_runtime"))))]
            pub async fn dial_with_tls_config(
                addr: impl ToSocketAddrs,
                domain: &str,
                config: ClientConfig
            ) -> Result<Self, Error> {
                tcp_client_with_tls_config(addr, domain, config).await
            }

            /// Connects to an HTTP RPC server at the specified network address using WebSocket and the defatul codec.
            ///
            /// It is recommended to use "ws://" as the url scheme as opposed to "http://"; however, internally the url scheme
            /// is changed to "ws://". Internally, `DEFAULT_RPC_PATH="_rpc"` is appended to the end of `addr`,
            /// and the rest is the same is calling `dial_websocket`.
            /// If a network path were to be supplpied, the network path must end with a slash "/".
            /// For example, a valid path could be "ws://127.0.0.1/rpc/".
            ///
            /// *Warning*: WebSocket is used as the underlying transport protocol starting from version "0.5.0-beta.0",
            /// and this will make client of versions later than "0.5.0-beta.0" incompatible with servers of versions
            /// earlier than "0.5.0-beta.0".
            ///
            /// This is enabled
            /// if and only if **only one** of the the following feature flag is turned on
            /// - `serde_bincode`
            /// - `serde_json`
            /// - `serde_cbor`
            /// - `serde_rmp`
            ///
            /// # Example
            ///
            /// ```rust
            /// let addr = "ws://127.0.0.1:8080/rpc/";
            /// let client = Client::dial_http(addr).await.unwrap();
            /// ```
            ///
            #[cfg_attr(feature = "docs", doc(cfg(feature = "tokio_runtime")))]
            pub async fn dial_http(addr: &str) -> Result<Self, Error> {
                let mut url = url::Url::parse(addr)?.join(DEFAULT_RPC_PATH)?;
                url.set_scheme("ws").expect("Failed to change scheme to ws");

                Self::dial_websocket_url(url).await
            }

            /// Connects to an HTTP RPC server with TLS enabled
            ///
            /// An example with self-signed certificate can be found in the
            /// [GitHub repo](https://github.com/minghuaw/toy-rpc/blob/9793bf53909bd7ffa74967fae6267f973e03ec8a/examples/warp_tls/src/bin/client.rs#L25)
            #[cfg(feature = "tls")]
            #[cfg_attr(feature = "docs",doc(cfg(all(feature ="tls", feature = "tokio_runtime"))))]
            pub async fn dial_http_with_tls_config(
                addr: &str,
                domain: &str,
                config: ClientConfig,
            ) -> Result<Self, Error> {
                let mut url = url::Url::parse(addr)?.join(DEFAULT_RPC_PATH)?;
                url.set_scheme("ws").expect("Failed to change scheme to ws");

                websocket_client_with_tls_config(url, domain, config).await
            }

            /// Similar to `dial`, this connects to an WebSocket RPC server at the specified network address using the defatul codec
            ///
            /// The difference between `dial_websocket` and `dial_http` is that, `dial_websocket` does not
            /// append `DEFAULT_RPC_PATH="_rpc"` to the end of the addr.
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
            /// let addr = "ws://127.0.0.1:8080";
            /// let client = Client::dial_websocket(addr).await.unwrap();
            /// ```
            ///
            #[cfg_attr(feature = "docs", doc(cfg(feature = "tokio_runtime")))]
            pub async fn dial_websocket(addr: &str) -> Result<Self, Error> {
                let url = url::Url::parse(addr)?;
                Self::dial_websocket_url(url).await
            }

            /// Similar to `dial_websocket` but with TLS enabled
            #[cfg(feature = "tls")]
            #[cfg_attr(feature = "docs",doc(cfg(all(feature ="tls", feature = "tokio_runtime"))))]
            pub async fn dial_websocket_with_tls_config(
                addr: &str,
                domain: &str,
                config: ClientConfig,
            ) -> Result<Self, Error> {
                let url = url::Url::parse(addr)?;
                websocket_client_with_tls_config(url, domain, config).await
            }
        }
    }
}

pub mod call;
pub use call::Call;

/// RPC client
///
#[cfg_attr(
    not(any(feature = "async_std_runtime", feature = "tokio_runtime")),
    allow(dead_code)
)]
pub struct Client<AckMode> {
    count: Arc<AtomicMessageId>,
    default_timeout: Duration,
    next_timeout: AtomicCell<Option<Duration>>,
    broker: Sender<ClientBrokerItem>,
    subscriptions: HashMap<String, TypeId>,

    ack_mode: PhantomData<AckMode>,
}

// seems like it still works even without this impl
impl<AckMode> Drop for Client<AckMode> {
    fn drop(&mut self) {
        for (topic, _) in self.subscriptions.drain() {
            self.broker
                .try_send(broker::ClientBrokerItem::Unsubscribe { topic })
                .unwrap_or_else(|err| log::error!("{}", err));
        }

        if let Err(err) = self.broker.try_send(broker::ClientBrokerItem::Stop) {
            log::error!("Failed to send stop signal to writer loop: {}", err);
        }
    }
}

impl<AckMode> Client<AckMode> {
    /// Creates a `ClientBuilder`.
    pub fn builder() -> builder::ClientBuilder<AckModeNone> {
        builder::ClientBuilder::default()
    }

    /// Closes connection with the server
    ///
    /// Dropping the client will close the connection as well
    pub async fn close(mut self) {
        // log::debug!("Unsunscribe all");
        for (topic, _) in self.subscriptions.drain() {
            self.broker
                .send_async(broker::ClientBrokerItem::Unsubscribe { topic })
                .await
                .unwrap_or_else(|err| log::error!("{}", err));
        }

        self.broker
            .send_async(broker::ClientBrokerItem::Stop)
            .await
            .unwrap_or_else(|err| log::error!("{}", err));
    }
}

// =============================================================================
// Public functions
// =============================================================================

cfg_if! {
    if #[cfg(any(
        feature = "docs",
        all(feature = "async_std_runtime", not(feature = "tokio_runtime")),
        all(feature = "tokio_runtime", not(feature = "async_std_runtime"))
    ))] {
        use std::sync::atomic::Ordering;

        use crate::{
            codec::split::SplittableCodec,
            // message::{ClientRequestBody, RequestHeader},
        };
        use reader::*;
        use writer::*;

        impl<AckMode> Client<AckMode> {
            /// Creates an RPC 'Client` over socket with a specified codec
            ///
            /// Example
            ///
            /// ```rust
            /// let addr = "127.0.0.1:8080";
            /// let stream = TcpStream::connect(addr).await.unwrap();
            /// let codec = Codec::new(stream);
            /// let client = Client::with_codec(codec);
            /// ```
            // #[cfg(any(
            //     all(feature = "async_std_runtime", not(feature = "tokio_runtime")),
            //     all(feature = "tokio_runtime", not(feature = "async_std_runtime"))
            // ))]
            #[cfg_attr(feature = "docs", doc(cfg(all(feature = "async_std_runtime", not(feature = "tokio_runtime")))))]
            #[cfg_attr(feature = "docs", doc(cfg(all(feature = "tokio_runtime", not(feature = "async_std_runtime")))))]
            pub fn with_codec<C>(codec: C) -> Self
            where
                C: SplittableCodec + Send + 'static,
            {
                let (writer, reader) = codec.split();
                let reader = ClientReader { reader };
                let writer = ClientWriter { writer };
                let count = Arc::new(AtomicMessageId::new(0));

                let broker = broker::ClientBroker {
                    count: count.clone(),
                    pending: HashMap::new(),
                    next_timeout: None,
                    subscriptions: HashMap::new(),
                };
                let (_, broker) = brw::spawn(broker, reader, writer);

                Self {
                    count,
                    default_timeout: Duration::from_secs(DEFAULT_TIMEOUT_SECONDS),
                    next_timeout: AtomicCell::new(None),
                    broker,
                    subscriptions: HashMap::new(),

                    ack_mode: PhantomData
                }
            }
        }

        impl<AckMode> Client<AckMode> {
            /// Sets the default timeout duration for this client
            ///
            /// Example
            ///
            /// ```rust
            /// let call: Call<()> = client
            ///     .set_default_timeout(std::time::Duration::from_secs(2)) // the RPC Call will timeout after 2 seconds
            ///     .call("Service.wait_for_10secs", ()); // request a RPC call that waits for 10 seconds
            /// let result = call.await;
            /// println!("{:?}", result); // Err(Error::Timeout(Some(call_id)))
            /// ```
            #[cfg_attr(feature = "docs", doc(cfg(all(feature = "async_std_runtime", not(feature = "tokio_runtime")))))]
            #[cfg_attr(feature = "docs", doc(cfg(all(feature = "tokio_runtime", not(feature = "async_std_runtime")))))]
            pub fn set_default_timeout(&mut self, duration: Duration) -> &Self {
                self.default_timeout = duration;
                self
            }

            /// Sets the timeout duration **ONLY** for the next RPC request
            ///
            /// Example
            ///
            /// ```rust
            /// let call: Call<()> = client
            ///     .set_next_timeout(std::time::Duration::from_secs(2)) // the RPC Call will timeout after 2 seconds
            ///     .call("Service.wait_for_10secs", ()); // request a RPC call that waits for 10 seconds
            /// let result = call.await;
            /// println!("{:?}", result); // Err(Error::Timeout(Some(call_id)))
            /// ```
            #[cfg_attr(feature = "docs", doc(cfg(all(feature = "async_std_runtime", not(feature = "tokio_runtime")))))]
            #[cfg_attr(feature = "docs", doc(cfg(all(feature = "tokio_runtime", not(feature = "async_std_runtime")))))]
            pub fn set_next_timeout(&self, duration: Duration) -> &Self {
                let _ = self.next_timeout.swap(Some(duration));
                self
            }

            /// Invokes the named function and wait synchronously in a blocking manner.
            ///
            /// This function internally calls `task::block_on` to wait for the response.
            /// Do NOT use this function inside another `task::block_on`.async_std
            ///
            /// Example
            ///
            /// ```rust
            /// let args = "arguments";
            /// let reply: Result<String, Error> = client
            ///     .call_blocking("EchoService.echo", &args); // This is a blocking call and you dont need to .await
            /// println!("{:?}", reply);
            /// ```
            #[cfg_attr(feature = "docs", doc(cfg(all(feature = "async_std_runtime", not(feature = "tokio_runtime")))))]
            #[cfg_attr(feature = "docs", doc(cfg(all(feature = "tokio_runtime", not(feature = "async_std_runtime")))))]
            pub fn call_blocking<Req, Res>(
                &self,
                service_method: impl ToString,
                args: Req,
            ) -> Result<Res, Error>
            where
                Req: serde::Serialize + Send + Sync + 'static,
                Res: serde::de::DeserializeOwned + Send + 'static,
            {
                let call = self.call(service_method, args);

                #[cfg(all(
                    feature = "async_std_runtime",
                    not(feature = "tokio_runtime")
                ))]
                let res = ::async_std::task::block_on(call);


                #[cfg(all(
                    feature = "tokio_runtime",
                    not(feature = "async_std_runtime")
                ))]
                let res = ::tokio::task::block_in_place(|| {
                    futures::executor::block_on(call)
                });

                res
            }

            /// Invokes the named RPC function call asynchronously and returns a cancellation `Call`
            ///
            /// The `Call<Res>` type takes one type argument `Res` which is the type of successful RPC execution.
            /// The result can be obtained by `.await`ing on the `Call`, which returns type `Result<Res, toy_rpc::Error>`
            /// `Call` can be cancelled by calling the `cancel()` function.
            /// The request will be sent in a background task.
            ///
            /// Example
            ///
            /// ```rust
            /// // Get the result by `.await`ing on the `Call`
            /// let call: Call<i32> = client.call("SomeService.echo_i32", 7i32);
            /// let reply: Result<i32, toy_rpc::Error> = call.await;
            ///
            /// // Cancel the call
            /// let call: Call<()> = client.call("SomeService.infinite_loop", ());
            /// // cancel takes a reference
            /// // .await on a canceled `Call` will return `Err(Error::Canceled(Some(id)))`
            /// call.cancel();
            /// let reply = call.await;
            /// println!("This should be a Err(Error::Canceled) {:?}", reply);
            /// ```
            #[cfg_attr(feature = "docs", doc(cfg(all(feature = "async_std_runtime", not(feature = "tokio_runtime")))))]
            #[cfg_attr(feature = "docs", doc(cfg(all(feature = "tokio_runtime", not(feature = "async_std_runtime")))))]
            pub fn call<Req, Res>(&self, service_method: impl ToString, args: Req) -> Call<Res>
            where
                Req: serde::Serialize + Send + Sync + 'static,
                Res: serde::de::DeserializeOwned + Send + 'static,
            {
                // Prepare RPC request
                // let id = self.count.load(Ordering::Relaxed) as MessageId;
                let id = self.count.fetch_add(1, Ordering::Relaxed);
                let service_method = service_method.to_string();
                let duration = match self.next_timeout.swap(None) {
                    Some(dur) => dur,
                    None => self.default_timeout.clone()
                };
                let body = Box::new(args) as Box<OutboundBody>;
                let (resp_tx, resp_rx) = oneshot::channel();

                if let Err(err) = self.broker.send(
                    ClientBrokerItem::Request{
                        id,
                        service_method,
                        duration,
                        body,
                        resp_tx,
                    }
                ) {
                    log::error!("{:?}", err);
                }

                // Creates Call
                Call::<Res>::new(id, self.broker.clone(), resp_rx)
            }
        }
    }
}
