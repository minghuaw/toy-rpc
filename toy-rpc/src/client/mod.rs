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
use builder::ClientBuilder;

type ResponseResult = Result<Box<InboundBody>, Box<InboundBody>>;

cfg_if! {
    if #[cfg(any(
        feature = "docs",
        all(feature = "tokio_runtime", not(feature = "async_std_runtime")),
        all(feature = "async_std_runtime", not(feature = "tokio_runtime"))
    ))] {
        use futures::channel::oneshot;

        use crate::{Error, protocol::{OutboundBody}};

        const DEFAULT_TIMEOUT_SECONDS: u64 = 10;
    }
}

cfg_if! {
    if #[cfg(any(
        feature = "docs",
        all(feature = "tokio_runtime", not(feature = "async_std_runtime"))
    ))] {
        use tokio::net::ToSocketAddrs;
        use ::tokio::io::{AsyncRead, AsyncWrite};
        use tokio::task::JoinHandle;
    } else if #[cfg(all(feature = "async_std_runtime", not(feature = "tokio_runtime")))] {
        use async_std::net::ToSocketAddrs;
        use futures::{AsyncRead, AsyncWrite};
        use async_std::task::JoinHandle;
    }
}

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
    broker_handle: Option<JoinHandle<()>>,
    subscriptions: HashMap<String, TypeId>,

    ack_mode: PhantomData<AckMode>,
}

cfg_if! {
    if #[cfg(any(
        feature = "docs",
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
    ))] {
        #[cfg(feature = "tls")]
        use rustls::{ClientConfig};

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
                ClientBuilder::default().dial(addr).await
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
                ClientBuilder::default().dial_with_tls_config(addr, domain, config).await
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
            #[cfg(any(feature = "ws_tokio", feature = "ws_async_std"))]
            #[cfg_attr(feature = "docs", doc(cfg(any(feature = "ws_tokio", feature = "ws_async_std"))))]
            pub async fn dial_http(addr: &str) -> Result<Self, Error> {
                ClientBuilder::default().dial_http(addr).await
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
                ClientBuilder::default().dial_http_with_tls_config(addr, domain, config).await
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
            #[cfg(any(feature = "ws_tokio", feature = "ws_async_std"))]
            #[cfg_attr(feature = "docs", doc(cfg(any(feature = "ws_tokio", feature = "ws_async_std"))))]
            pub async fn dial_websocket(addr: &str) -> Result<Self, Error> {
                ClientBuilder::default().dial_websocket(addr).await
            }

            /// Similar to `dial_websocket` but with TLS enabled
            #[cfg(feature = "tls")]
            #[cfg_attr(feature = "docs",doc(cfg(all(feature ="tls", feature = "tokio_runtime"))))]
            pub async fn dial_websocket_with_tls_config(
                addr: &str,
                domain: &str,
                config: ClientConfig,
            ) -> Result<Self, Error> {
                ClientBuilder::default().dial_websocket_with_tls_config(addr, domain, config).await
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
                let builder = ClientBuilder::default();
                builder.with_stream(stream)
            }

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
                let builder = ClientBuilder::default();
                builder.with_codec(codec)
            }
        }
    }
}

pub mod call;
pub use call::Call;

// seems like it still works even without this impl
impl<AckMode> Drop for Client<AckMode> {
    fn drop(&mut self) {
        if !self.broker.is_disconnected() {
            for (topic, _) in self.subscriptions.drain() {
                self.broker
                    .try_send(broker::ClientBrokerItem::Unsubscribe { topic })
                    .unwrap_or_else(|err| log::error!("{}", err));
            }

            if let Err(err) = self.broker.try_send(broker::ClientBrokerItem::Stopping) {
                log::error!("Failed to send stop signal to writer loop: {}", err);
            }
        }

        if let Some(handle) = self.broker_handle.take() {
            let _ = futures::executor::block_on(handle);
        }
    }
}

impl Client<AckModeNone> {
    /// Creates a `ClientBuilder`.
    pub fn builder() -> ClientBuilder<AckModeNone> {
        ClientBuilder::default()
    }
}

impl<AckMode> Client<AckMode> {
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
            .send_async(broker::ClientBrokerItem::Stopping)
            .await
            .unwrap_or_else(|err| log::error!("{}", err));

        if let Some(handle) = self.broker_handle.take() {
            let _ = handle.await;
        }
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

        use crate::{codec::split::SplittableCodec};

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
            /// println!("{:?}", result); // Err(Error::Timeout(call_id))
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
            /// println!("{:?}", result); // Err(Error::Timeout(call_id))
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
                    log::error!("{}", err);
                }

                // Creates Call
                Call::<Res>::new(id, self.broker.clone(), resp_rx)
            }
        }
    }
}
