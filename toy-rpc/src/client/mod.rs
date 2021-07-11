//! RPC Client impementation

use cfg_if::cfg_if;
use crossbeam::atomic::AtomicCell;
use flume::Sender;
use futures::{Future, Sink, channel::oneshot};
use std::{any::TypeId, marker::PhantomData, pin::Pin, sync::Arc, task::{Context, Poll}};

use crate::{Error, message::{AtomicMessageId, MessageId}, protocol::OutboundBody, pubsub::Topic};
use crate::protocol::InboundBody;

mod pubsub;
mod broker;
mod reader;
mod writer;

use broker::*;
use pubsub::*;

const DEFAULT_TIMEOUT_SECONDS: u64 = 10;

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
        #[cfg(any(feature = "async_std_runtime", feature = "tokio_runtime"))]
        use crate::codec::DefaultCodec;

        #[cfg(feature = "tls")]
        use rustls::{ClientConfig};
        #[cfg(feature = "tls")]
        use webpki::DNSNameRef;
        #[cfg(feature = "tls")]
        use crate::transport::ws::WebSocketConn;

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

        #[cfg(feature = "tls")]
        async fn tcp_client_with_tls_config(
            addr: impl ToSocketAddrs,
            domain: &str,
            config: ClientConfig
        ) -> Result<Client, Error> {
            let stream = TcpStream::connect(addr).await?;
            let connector = TlsConnector::from(std::sync::Arc::new(config));
            let domain = DNSNameRef::try_from_ascii_str(domain)?;
            let tls_stream = connector.connect(domain, stream).await?;

            Ok(Client::with_stream(tls_stream))
        }

        #[cfg(feature = "tls")]
        async fn websocket_client_with_tls_config(
            url: url::Url,
            domain: &str,
            config: ClientConfig,
        ) -> Result<Client, Error> {
            let host = url.host_str()
                .ok_or(Error::Internal("Invalid host address".into()))?;
            let port = url.port_or_known_default()
                .ok_or(Error::Internal("Invalid port".into()))?;
            let addr = (host, port);
            let stream = TcpStream::connect(addr).await?;
            let connector = TlsConnector::from(std::sync::Arc::new(config));
            let domain = DNSNameRef::try_from_ascii_str(domain)?;
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
    all(
        feature = "async_std_runtime",
        not(feature = "tokio_runtime")
    )
))]
mod async_std;
#[cfg(any(
    feature = "docs",
    all(
        feature = "tokio_runtime",
        not(feature = "async_std_runtime")
    )
))]mod tokio;

/* -------------------------------------------------------------------------- */
/*                                  Call                                      */
/* -------------------------------------------------------------------------- */

/// Call of a RPC request. The result can be obtained by `.await`ing the `Call`.
/// The call can be cancelled with `cancel()` method.
///
/// The type parameter `Res` is the `Ok` type of the result. `.await`ing on the `Call<Res>`
/// will yield a `Result<Res, toy_rpc::Error>`.
///
/// # Example
///
/// ```rust
/// // `.await` to wait for the response
/// let call: Call<i32> = client.call("Arith.add", (1i32, 6i32));
/// let result = call.await;
///
/// // cancel the call regardless of whether the response is received or not
/// let call: Call<()> = client.call("Arith.infinite_loop", ());
/// call.cancel();
/// // You can still .await on the canceled `Call` but will get an error
/// let result = call.await; // Err(Error::Canceled(Some(id)))
/// ```
// #[pin_project::pin_project(PinnedDrop)]
#[pin_project::pin_project]
pub struct Call<Res> {
    id: MessageId,
    cancel: Sender<broker::ClientBrokerItem>,
    #[pin]
    done: oneshot::Receiver<Result<Result<Box<InboundBody>, Box<InboundBody>>, Error>>,
    marker: PhantomData<Res>,
}

impl<Res> Call<Res>
where
    Res: serde::de::DeserializeOwned,
{
    /// Cancel the RPC call
    ///
    pub fn cancel(&self) {
        match self.cancel.send(broker::ClientBrokerItem::Cancel(self.id)) {
            Ok(_) => {
                log::info!("Call is canceled");
            }
            Err(_) => {
                log::error!("Failed to cancel")
            }
        }
    }

    /// Gets the ID number of the call
    ///
    /// Each client RPC call has a monotonically increasing ID number of type `u16`
    pub fn get_id(&self) -> MessageId {
        self.id
    }
}

impl<Res> Future for Call<Res>
where
    Res: serde::de::DeserializeOwned,
{
    type Output = Result<Res, Error>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        let done: Pin<&mut oneshot::Receiver<Result<Result<Box<InboundBody>, Box<InboundBody>>, Error>>> = this.done;

        match done.poll(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(res) => {
                let res = match res {
                    Ok(val) => val,
                    Err(_canceled) => return Poll::Ready(Err(Error::Canceled(Some(*this.id)))),
                };

                let res = match res {
                    Ok(val) => val,
                    Err(err) => return Poll::Ready(Err(err))
                };

                let res = match res {
                    Ok(mut resp_body) => erased_serde::deserialize(&mut resp_body)
                        .map_err(|err| Error::ParseError(Box::new(err))),
                    Err(mut err_body) => erased_serde::deserialize(&mut err_body).map_or_else(
                        |err| Err(Error::ParseError(Box::new(err))),
                        |msg| Err(Error::from_err_msg(msg)),
                    ),
                };
                Poll::Ready(res)
            },
        }
    }
}

/* -------------------------------------------------------------------------- */
/*                               RPC client                                   */
/* -------------------------------------------------------------------------- */

/// RPC client
///
#[cfg_attr(
    not(any(feature = "async_std_runtime", feature = "tokio_runtime")),
    allow(dead_code)
)]
pub struct Client {
    count: Arc<AtomicMessageId>,
    timeout: AtomicCell<Duration>,
    broker: Sender<ClientBrokerItem>,
    subscriptions: HashMap<String, TypeId>,
}

// seems like it still works even without this impl
impl Drop for Client {
    fn drop(&mut self) {
        if self.broker.try_send(broker::ClientBrokerItem::Stop).is_err() {
            log::error!("Failed to send stop signal to writer loop")
        }
    }
}

impl Client {
    /// Closes connection with the server
    ///
    /// Dropping the client will close the connection as well
    pub async fn close(self) {
        self.broker.send_async(
            broker::ClientBrokerItem::Stop
        ).await
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
        use std::collections::HashMap;
        use std::time::Duration;

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

                let broker = ClientBroker {
                    count: count.clone(),
                    pending: HashMap::new(),
                    next_timeout: None,
                    subscriptions: HashMap::new(),
                };          
                let (_, broker) = brw::spawn(broker, reader, writer);

                Client {
                    count,
                    timeout: AtomicCell::new(Duration::from_secs(DEFAULT_TIMEOUT_SECONDS)),
                    broker,
                    subscriptions: HashMap::new(),
                }
            }
        }

        impl Client {
            /// Sets the timeout duration **ONLY** for the next RPC request
            ///
            /// Example
            ///
            /// ```rust
            /// let call: Call<()> = client
            ///     .timeout(std::time::Duration::from_secs(2)) // the RPC Call will timeout after 2 seconds
            ///     .call("Service.wait_for_10secs", ()); // request a RPC call that waits for 10 seconds
            /// let result = call.await; 
            /// println!("{:?}", result); // Err(Error::Timeout(Some(call_id)))
            /// ```
            #[cfg_attr(feature = "docs", doc(cfg(all(feature = "async_std_runtime", not(feature = "tokio_runtime")))))]
            #[cfg_attr(feature = "docs", doc(cfg(all(feature = "tokio_runtime", not(feature = "async_std_runtime")))))]
            pub fn timeout(&self, duration: Duration) -> &Self {
                // self.broker.send(ClientBrokerItem::SetTimeout(duration))
                //     .unwrap_or_else(|err| log::error!("{:?}", err));
                self.timeout.store(duration);
                &self
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
                // let id = self.count.fetch_add(1, Ordering::Relaxed);
                let id = self.count.load(Ordering::Relaxed) as MessageId;
                let service_method = service_method.to_string();
                let duration = self.timeout.load();
                // let header = Header::Request { id, service_method, timeout};
                let body = Box::new(args) as Box<OutboundBody>;
                let (resp_tx, resp_rx) = oneshot::channel();

                if let Err(err) = self.broker.send(
                    ClientBrokerItem::Request{
                        // id,
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

            /// Creates a new publisher on a topic. 
            ///
            /// Multiple local publishers on the same topic are allowed. 
            pub fn publisher<T>(&self) -> Publisher<T> 
            where 
                T: Topic,
            {
                let tx = self.broker.clone();
                Publisher::from(tx)
            }

            /// Creates a new subscriber on a topic
            ///
            pub fn subscriber<T: Topic + 'static>(&mut self, cap: usize) -> Result<Subscriber<T>, Error> {
                let (tx, rx) = flume::bounded(cap);
                let topic = T::topic();

                // Check if there is an existing subscriber
                if self.subscriptions.contains_key(&topic) {
                    return Err(Error::Internal("Only one local subscriber per topic is allowed".into()))
                }
                self.subscriptions.insert(topic.clone(), TypeId::of::<T>());
                
                // Create new subscription
                if let Err(err) = self.broker.send(ClientBrokerItem::Subscribe{topic, item_sink: tx}) {
                    return Err(err.into())
                };

                let sub = Subscriber::from_recver(rx);
                Ok(sub)
            }

            /// Replaces the local subscriber without sending any message to the server
            ///
            /// The previous subscriber will no longer receive any message.
            pub fn replace_local_subscriber<T: Topic + 'static>(&mut self, cap: usize) -> Result<Subscriber<T>, Error> {
                let topic = T::topic();
                match self.subscriptions.get(&topic) {
                    Some(entry) => {
                        match &TypeId::of::<T>() == entry {
                            true => {
                                let (tx, rx) = flume::bounded(cap);
                                if let Err(err) = self.broker.send(ClientBrokerItem::NewLocalSubscriber{topic, new_item_sink: tx}) {
                                    return Err(err.into())
                                }
                                let sub = Subscriber::from_recver(rx);
                                Ok(sub)
                            },
                            false => Err(Error::Internal("TypeId mismatch".into()))
                        }
                    },
                    None => Err(Error::Internal("There is no existing local subscriber".into()))
                }
            }
        }
    }
}