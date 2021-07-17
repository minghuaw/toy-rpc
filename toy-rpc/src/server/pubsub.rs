use std::collections::{HashMap, BTreeMap};
use std::marker::PhantomData;
use std::sync::Arc;
use std::pin::Pin;
use std::sync::atomic::Ordering;
use std::task::{Context, Poll};
use flume::r#async::{RecvStream, SendSink};
use flume::{Sender, Receiver};
use futures::{Stream, Sink};
use pin_project::pin_project;

#[cfg(feature = "http_actix_web")]
use actix::Recipient;

use crate::codec::{Marshal, Reserved, Unmarshal};
use crate::message::{AtomicMessageId, MessageId};
use crate::pubsub::{Topic};
use crate::error::Error;

#[cfg(not(feature = "http_actix_web"))]
use super::RESERVED_CLIENT_ID;
use super::{broker::ServerBrokerItem, ClientId, Server};

pub(crate) enum PubSubItem {
    Publish {
        msg_id: MessageId,
        topic: String,
        content: Arc<Vec<u8>>,
    },
    Subscribe {
        client_id: ClientId,
        topic: String,
        #[cfg(not(feature = "http_actix_web"))]
        sender: Sender<ServerBrokerItem>,
        #[cfg(feature = "http_actix_web")]
        sender: Recipient<ServerBrokerItem>,
    },
    Unsubscribe {
        client_id: ClientId,
        topic: String,
    },
    Stop
}

pub(crate) struct PubSubBroker {
    listener: Receiver<PubSubItem>,
    
    #[cfg(feature = "http_actix_web")]
    subscriptions: HashMap<String, BTreeMap<ClientId, Recipient<ServerBrokerItem>>>,
    
    #[cfg(not(feature = "http_actix_web"))]
    subscriptions: HashMap<String, BTreeMap<ClientId, Sender<ServerBrokerItem>>>,
}

impl PubSubBroker {
    pub fn new(listener: Receiver<PubSubItem>) -> Self {
        Self {
            listener,
            subscriptions: HashMap::new()
        }
    }

    /// Spawn PubSubBroker loop in a task
    #[cfg(any(
        all(feature = "async_std_runtime", not(feature = "tokio_runtime")),
        all(feature = "tokio_runtime", not(feature = "async_std_runtime")),
    ))]
    pub fn spawn(self) {
        #[cfg(all(feature = "async_std_runtime", not(feature = "tokio_runtime")))]
        ::async_std::task::spawn(self.pubsub_loop());
        #[cfg(all(feature = "tokio_runtime", not(feature = "async_std_runtime"), not(feature = "http_actix_web")))]
        ::tokio::task::spawn(self.pubsub_loop());
        #[cfg(all(feature = "http_actix_web", not(feature = "async_std_runtime")))]
        actix::spawn(self.pubsub_loop());
    }

    pub async fn pubsub_loop(mut self) {
        while let Ok(item) = self.listener.recv_async().await {
            match item {
                PubSubItem::Publish {msg_id, topic, content} => {
                    if let Some(entry) = self.subscriptions.get_mut(&topic) {
                        entry.retain(|_, sender| {
                            let msg = ServerBrokerItem::Publication{
                                id: msg_id, 
                                topic: topic.clone(), 
                                content: content.clone()
                            };
                            if let Err(err) = sender.try_send(msg) {
                                #[cfg(not(feature = "http_actix_web"))]
                                match err {
                                    flume::TrySendError::Disconnected(_) => {
                                        log::error!("Client is disconnected, removing from subscriptions");
                                        return false
                                    }
                                    e @ _ => log::error!("{}", e)
                                }
                                
                                #[cfg(feature = "http_actix_web")]
                                match err {
                                    actix::prelude::SendError::Closed(_) => {
                                        log::error!("Client is disconnected, removing from subscriptions");
                                        return false
                                    }
                                    e @ _ => log::error!("{}", e)
                                }
                            }
                            true
                        })
                    }
                },
                PubSubItem::Subscribe {client_id, topic, sender} => {
                    match self.subscriptions.get_mut(&topic) {
                        Some(entry) => {
                            entry.insert(client_id, sender);
                        },
                        None => {
                            let mut entry = BTreeMap::new();
                            entry.insert(client_id, sender);
                            self.subscriptions.insert(topic, entry);
                        }
                    }
                },
                PubSubItem::Unsubscribe {client_id, topic} => {
                    match self.subscriptions.get_mut(&topic) {
                        Some(entry) => {
                            entry.remove(&client_id);
                        },
                        None => { }
                    }
                },
                PubSubItem::Stop => {
                    return
                }
            }
        }
    }
}

/* -------------------------------------------------------------------------- */
/*                                 Public API                                 */
/* -------------------------------------------------------------------------- */

#[pin_project]
pub struct Publisher<T: Topic, C: Marshal> {
    #[pin]
    inner: SendSink<'static, PubSubItem>,
    counter: AtomicMessageId,
    marker: PhantomData<T>,
    codec: PhantomData<C>
}

impl<T: Topic, C: Marshal> From<Sender<PubSubItem>> for Publisher<T, C> {
    fn from(inner: Sender<PubSubItem>) -> Self {
        Self {
            inner: inner.into_sink(),
            counter: AtomicMessageId::new(0),
            marker: PhantomData,
            codec: PhantomData,
        }
    }
}

impl<T: Topic, C: Marshal> Sink<T::Item> for Publisher<T, C> {
    type Error = Error; 

    fn poll_ready(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Result<(), Self::Error>> {
        let this = self.project();
        this.inner.poll_ready(cx)
            .map_err(|err| err.into())
    }

    fn start_send(self: Pin<&mut Self>, item: T::Item) -> Result<(), Self::Error> {
        let this = self.project();
        let topic = T::topic();
        let msg_id = this.counter.fetch_add(1, Ordering::Relaxed);
        let body = C::marshal(&item)?;
        let content = Arc::new(body);
        let item = PubSubItem::Publish{msg_id, topic, content};
        this.inner.start_send(item)
            .map_err(|err| err.into())
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Result<(), Self::Error>> {
        let this = self.project();
        this.inner.poll_flush(cx)
            .map_err(|err| err.into())
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Result<(), Self::Error>> {
        let this = self.project();
        this.inner.poll_close(cx)
            .map_err(|err| err.into())
    }
}

#[pin_project]
pub struct Subscriber<T: Topic, C: Unmarshal> {
    #[pin]
    inner: RecvStream<'static, ServerBrokerItem>,
    topic: String,
    marker: PhantomData<T>,
    codec: PhantomData<C>
} 

impl<T: Topic, C: Unmarshal> From<Receiver<ServerBrokerItem>> for Subscriber<T, C> {
    fn from(inner: Receiver<ServerBrokerItem>) -> Self {
        Self {
            inner: inner.into_stream(),
            topic: T::topic(),
            marker: PhantomData,
            codec: PhantomData,
        }
    }
}

impl<T: Topic, C: Unmarshal> Stream for Subscriber<T, C> {
    type Item = Result<T::Item, Error>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.project();
        match this.inner.poll_next(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(opt) => {
                match opt {
                    Some(item) => {
                        match item {
                            ServerBrokerItem::Publication{id: _, topic, content} => {
                                let result = match &topic == this.topic {
                                    true => {
                                        C::unmarshal(&content)
                                    },
                                    false => {
                                        Err(Error::Internal("Mismatched topic".into()))
                                    }
                                };
                                Poll::Ready(Some(result))
                            },
                            _ => {
                                let result = Err(Error::Internal("Invalid PubSub item".into()));
                                Poll::Ready(Some(result))
                            }
                        }
                    },
                    None => Poll::Ready(None)
                }
            }
        }
    }
}


cfg_if::cfg_if!{
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
        use crate::codec::DefaultCodec;

        type PhantomCodec = DefaultCodec<Reserved, Reserved, Reserved>;
        
        impl Server {
            /// Creates a new publihser on a topic
            pub fn publisher<T: Topic>(&self) -> Publisher<T, PhantomCodec> {
                let tx = self.pubsub_tx.clone();
                Publisher::from(tx)
            }

            /// Creates a new subscriber on a topic
            ///
            /// Multiple subscribers can be created on the server side
            #[cfg(not(feature = "http_actix_web"))]
            #[cfg_attr(feature = "docs", doc(cfg(not(feature = "http_actix_web"))))]
            pub fn subscriber<T: Topic>(&self, cap: usize) -> Result<Subscriber<T, PhantomCodec>, Error> {
                let (sender, rx) = flume::bounded(cap);
                let client_id = RESERVED_CLIENT_ID;
                let topic = T::topic();
                self.pubsub_tx.send(PubSubItem::Subscribe{client_id, topic, sender})?;
                Ok(
                    Subscriber::from(rx)
                )
            }
        }
    }
}
