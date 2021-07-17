//! RPC Client impementation

use cfg_if::cfg_if;
use crossbeam::atomic::AtomicCell;
use flume::Sender;
use std::{any::TypeId, collections::HashMap, sync::Arc, time::Duration};

use crate::message::AtomicMessageId;

pub(crate) mod broker;
mod pubsub;
mod reader;
mod writer;

use broker::ClientBrokerItem;

cfg_if! {
    if #[cfg(any(
        feature = "docs",
        all(feature = "tokio_runtime", not(feature = "async_std_runtime")),
        all(feature = "async_std_runtime", not(feature = "tokio_runtime"))
    ))] {
        use futures::channel::oneshot;
        use std::marker::PhantomData;
        use crate::{Error, message::{MessageId}, protocol::OutboundBody};

        #[cfg(feature = "tls")]
        use crate::transport::ws::WebSocketConn;

        #[cfg(feature = "tls")]
        use crate::codec::DefaultCodec;

        const DEFAULT_TIMEOUT_SECONDS: u64 = 10;
    }
}

cfg_if! {
    if #[cfg(any(
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
        )
    ))] {
        #[cfg(all(
            feature = "tls",
            all(feature = "tokio_runtime", not(feature = "async_std_runtime"))
        ))]
        use ::tokio::net::{TcpStream, ToSocketAddrs};
        #[cfg(all(
            feature = "tls",
            all(feature = "tokio_runtime", not(feature = "async_std_runtime"))
        ))]
        use tokio_rustls::TlsConnector;
        #[cfg(all(
            feature = "tls",
            all(feature = "tokio_runtime", not(feature = "async_std_runtime"))
        ))]
        use async_tungstenite::tokio::client_async;

        #[cfg(all(
            feature = "tls",
            all(feature = "async_std_runtime", not(feature = "tokio_runtime"))
        ))]
        use ::async_std::net::{TcpStream, ToSocketAddrs};
        #[cfg(all(
            feature = "tls",
            all(feature = "async_std_runtime", not(feature = "tokio_runtime"))
        ))]
        use async_rustls::TlsConnector;
        #[cfg(all(
            feature = "tls",
            all(feature = "async_std_runtime", not(feature = "tokio_runtime"))
        ))]
        use async_tungstenite::client_async;

        #[cfg(all(
            feature = "tls",
            any(
                all(feature = "tokio_runtime", not(feature = "async_std_runtime")),
                all(feature = "async_std_runtime", not(feature = "tokio_runtime"))
            )
        ))]
        async fn tcp_client_with_tls_config(
            addr: impl ToSocketAddrs,
            domain: &str,
            config: rustls::ClientConfig
        ) -> Result<Client, Error> {
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
        async fn websocket_client_with_tls_config(
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
            let domain = webpki::DNSNameRef::try_from_ascii_str(domain)?;
            let tls_stream = connector.connect(domain, stream).await?;
            let (ws_stream, _) = client_async(url, tls_stream).await?;
            let ws_stream = WebSocketConn::new(ws_stream);
            let codec = DefaultCodec::with_websocket(ws_stream);
            Ok(Client::with_codec(codec))
        }
    }
}

#[cfg(any(
    feature = "docs",
    all(feature = "async_std_runtime", not(feature = "tokio_runtime"))
))]
mod async_std;
#[cfg(any(
    feature = "docs",
    all(feature = "tokio_runtime", not(feature = "async_std_runtime"))
))]
mod tokio;

pub mod call;
pub use call::Call;

/// RPC client
///
#[cfg_attr(
    not(any(feature = "async_std_runtime", feature = "tokio_runtime")),
    allow(dead_code)
)]
pub struct Client {
    count: Arc<AtomicMessageId>,
    default_timeout: Duration,
    next_timeout: AtomicCell<Option<Duration>>,
    broker: Sender<ClientBrokerItem>,
    subscriptions: HashMap<String, TypeId>,
}

// seems like it still works even without this impl
impl Drop for Client {
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

impl Client {
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

        impl Client {
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
            pub fn with_codec<C>(codec: C) -> Client
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

                Client {
                    count,
                    default_timeout: Duration::from_secs(DEFAULT_TIMEOUT_SECONDS),
                    next_timeout: AtomicCell::new(None),
                    broker,
                    subscriptions: HashMap::new(),
                }
            }
        }

        impl Client {
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
                let id = self.count.load(Ordering::Relaxed) as MessageId;
                let service_method = service_method.to_string();
                let duration = match self.next_timeout.swap(None) {
                    Some(dur) => dur,
                    None => self.default_timeout.clone()
                };
                let body = Box::new(args) as Box<OutboundBody>;
                let (resp_tx, resp_rx) = oneshot::channel();

                if let Err(err) = self.broker.send(
                    ClientBrokerItem::Request{
                        service_method,
                        duration,
                        body,
                        resp_tx,
                    }
                ) {
                    log::error!("{:?}", err);
                }

                // Creates Call
                Call::<Res> {
                    id,
                    cancel: self.broker.clone(),
                    done: resp_rx,
                    marker: PhantomData
                }
            }
        }
    }
}
