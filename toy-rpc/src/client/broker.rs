use async_trait::async_trait;
use cfg_if::cfg_if;
use flume::Sender;
use futures::channel::oneshot;
use std::{marker::PhantomData, time::Duration};

cfg_if! {
    if #[cfg(any(
        feature = "docs",
        all(feature = "tokio_runtime", not(feature = "async_std_runtime")),
        all(feature = "async_std_runtime", not(feature = "tokio_runtime"))
    ))] {
        use std::{sync::{Arc, atomic::Ordering}, collections::{HashMap, BTreeMap}};
        use futures::{SinkExt};

        use crate::message::AtomicMessageId;

        use super::{writer::ClientWriterItem};
    }
}

use crate::{
    codec::Marshal,
    message::MessageId,
    protocol::{InboundBody, OutboundBody},
    pubsub::SeqId,
    Error, util::Broker,
};

use super::{pubsub::SubscriptionItem, ResponseResult};

#[cfg_attr(
    all(not(feature = "tokio_runtime"), not(feature = "async_std_runtime")),
    allow(dead_code)
)]
pub(crate) enum ClientBrokerItem {
    Request {
        id: MessageId,
        service_method: String,
        duration: Duration,
        body: Box<OutboundBody>,
        resp_tx: oneshot::Sender<Result<ResponseResult, Error>>,
    },
    Response {
        id: MessageId,
        result: ResponseResult,
    },
    Cancel(MessageId),
    /// New publication to the server
    Publish {
        topic: String,
        body: Box<OutboundBody>,
    },
    PublishRetry {
        count: u32,
        id: MessageId,
        topic: String,
        body: Arc<Vec<u8>>,
    },
    Subscribe {
        topic: String,

        // message is deserialized as it is read on the subscriber
        item_sink: Sender<SubscriptionItem>,
    },
    NewLocalSubscriber {
        topic: String,
        new_item_sink: Sender<SubscriptionItem>,
    },
    Unsubscribe {
        // id: MessageId,
        topic: String,
    },
    /// Subscription from the server
    Subscription {
        id: SeqId,
        topic: String,
        item: Box<InboundBody>,
    },
    /// Ack reply from server
    InboundAck(SeqId),
    /// (Manual) Ack reply for incoming Publish message
    // OutboundAck(SeqId),

    /// Begin the stop process
    // #[cfg(any(feature = "ws_tokio", feature = "ws_async_std"))]
    Stopping,

    /// Stop
    ///
    ///
    Stop,
}

enum ClientBrokerState {
    Started,
    Stopping,
    Stopped,
}

#[cfg(all(feature = "async_std_runtime", not(feature = "tokio_runtime")))]
use ::async_std::task::{self};
#[cfg(all(feature = "tokio_runtime", not(feature = "async_std_runtime")))]
use ::tokio::task::{self};

#[cfg(any(
    feature = "docs",
    all(feature = "tokio_runtime", not(feature = "async_std_runtime")),
    all(feature = "async_std_runtime", not(feature = "tokio_runtime"))
))]
pub(crate) struct ClientBroker<C> {
    state: ClientBrokerState,
    pub count: Arc<AtomicMessageId>,
    pub pending: HashMap<MessageId, oneshot::Sender<Result<ResponseResult, Error>>>,
    pub subscriptions: HashMap<String, Sender<SubscriptionItem>>,
    pub pending_acks: BTreeMap<MessageId, oneshot::Sender<()>>,
    pub pub_retry_timeout: Duration,
    pub max_num_retries: u32,

    pub codec: PhantomData<C>,
}

#[cfg(any(
    feature = "docs",
    all(feature = "tokio_runtime", not(feature = "async_std_runtime")),
    all(feature = "async_std_runtime", not(feature = "tokio_runtime"))
))]
impl<C> ClientBroker<C> {
    pub fn new(
        count: Arc<AtomicMessageId>,
        pub_retry_timeout: Duration,
        max_num_retries: u32,
    ) -> Self {
        Self {
            state: ClientBrokerState::Started,
            count,
            pending: HashMap::new(),
            subscriptions: HashMap::new(),
            pending_acks: BTreeMap::new(),
            pub_retry_timeout,
            max_num_retries,

            // ack_mode: PhantomData,
            codec: PhantomData,
        }
    }

    async fn handle_request(
        &mut self,
        id: MessageId,
        service_method: String,
        duration: Duration,
        body: Box<OutboundBody>,
        resp_tx: oneshot::Sender<Result<ResponseResult, Error>>,
    ) -> Result<Option<ClientWriterItem>, Error>{
        // fetch_add returns the previous value
        let (tx, rx) = oneshot::channel();
        let fut = async move {
            // takes care of receiving/cancel  error
            match rx.await {
                Ok(res) => res,
                Err(_) => Err(Error::Canceled(id)),
            }
        };
        let item = ClientWriterItem::Request(id, service_method, duration, body);

        task::spawn(async move {
            #[cfg(all(feature = "tokio_runtime", not(feature = "async_std_runtime")))]
            let timout_result = ::tokio::time::timeout(duration, fut).await;
            #[cfg(all(feature = "async_std_runtime", not(feature = "tokio_runtime")))]
            let timout_result = ::async_std::future::timeout(duration, fut).await;

            let cancellation_result = match timout_result {
                Ok(res) => res,
                Err(_) => {
                    if let Err(_) = resp_tx.send(Err(Error::Timeout(id))) {
                        log::trace!("InternalError: Unable to send Error::Timeout({}) over response channel, response receiver is dropped", id);
                    }
                    return;
                }
            };
            match cancellation_result {
                Ok(res) => {
                    let response_result = Ok(res);
                    resp_tx.send(response_result)
                        .unwrap_or_else(|_| log::trace!("InternalError: Unable to send RPC response over response channel, response receiver is dropped"));
                }
                Err(_) => {
                    // RPC request is already canceled, simply return
                    return;
                }
            };
        });

        self.pending.insert(id, tx);
        Ok(Some(item))
    }

    fn handle_response(&mut self, id: MessageId, result: ResponseResult) -> Result<Option<ClientWriterItem>, Error> {
        if let Some(tx) = self.pending.remove(&id) {
            tx.send(Ok(result)).map_err(|_| {
                Error::Internal("InternalError: client failed to send response over channel".into())
            }).map(|_| None)
        } else {
            Err(Error::Internal(
                format!("InternalError: Response channel not found for id: {}", id).into(),
            ))
        }
    }

    async fn handle_cancel(
        &mut self,
        id: MessageId,
    ) -> Result<Option<ClientWriterItem>, Error> {
        if let Some(tx) = self.pending.remove(&id) {
            tx.send(Err(Error::Canceled(id))).map_err(|_| {
                Error::Internal(
                    format!(
                        "Unable to send Error::Canceled(Some({})) over response channel",
                        id
                    )
                    .into(),
                )
            })?;
        }

        Ok(Some(ClientWriterItem::Cancel(id)))
    }

    fn spawn_timed_task_waiting_for_ack(
        &mut self,
        broker: Sender<ClientBrokerItem>,
        count: u32,
        id: MessageId,
        topic: String,
        body: Arc<Vec<u8>>,
        duration: Duration,
    ) {
        // fetch_add returns the previous value
        let (tx, rx) = oneshot::channel::<()>();
        task::spawn(async move {
            #[cfg(all(feature = "tokio_runtime", not(feature = "async_std_runtime")))]
            let timeout_result = ::tokio::time::timeout(duration, rx).await;
            #[cfg(all(feature = "async_std_runtime", not(feature = "tokio_runtime")))]
            let timeout_result = ::async_std::future::timeout(duration, rx).await;

            // retry
            if let Err(_) = timeout_result {
                log::debug!("Publish ack timedout");
                broker
                    .send_async(ClientBrokerItem::PublishRetry {
                        count,
                        id,
                        topic,
                        body,
                    })
                    .await
                    .unwrap_or_else(|_| log::error!("Error found sending PublishRetry"))
            }
        });
        self.pending_acks.insert(id, tx);
    }

    async fn handle_publish_retry(
        &mut self,
        tx: &Sender<ClientBrokerItem>,
        mut count: u32,
        id: MessageId,
        topic: String,
        body: Arc<Vec<u8>>,
    ) -> Result<Option<ClientWriterItem>, Error> {
        if count < self.max_num_retries {
            count += 1;
            let item = ClientWriterItem::Publish(id, topic.clone(), body.clone());
            let broker = tx.clone();
            self.spawn_timed_task_waiting_for_ack(
                broker,
                count,
                id,
                topic,
                body,
                self.pub_retry_timeout,
            );
            Ok(Some(item))
        } else {
            Err(Error::MaxRetriesReached(id))
        }
    }

    async fn handle_subscribe(
        &mut self,
        tx: &Sender<ClientBrokerItem>,
        topic: String,
        item_sink: Sender<SubscriptionItem>,
    ) -> Result<Option<ClientWriterItem>, Error> {
        let id = self.count.fetch_add(1, Ordering::Relaxed);
        // NOTE: Only one local subscriber is allowed
        self.subscriptions.insert(topic.clone(), item_sink);

        Ok(Some(ClientWriterItem::Subscribe(id, topic)))
    }

    fn handle_new_local_subscriber(
        &mut self,
        topic: String,
        new_item_sink: Sender<SubscriptionItem>,
    ) -> Result<Option<ClientWriterItem>, Error> {
        self.subscriptions.insert(topic, new_item_sink);
        Ok(None)
    }

    async fn handle_unsubscribe(
        &mut self,
        topic: String,
    ) -> Result<Option<ClientWriterItem>, Error> {
        let id = self.count.fetch_add(1, Ordering::Relaxed);
        Ok(Some(ClientWriterItem::Unsubscribe(id, topic)))
    }

    fn handle_inbound_ack(&mut self, id: MessageId) -> Result<Option<ClientWriterItem>, Error> {
        if let Some(tx) = self.pending_acks.remove(&id) {
            tx.send(()).map_err(|_| {
                Error::Internal("InternalError: Failed to send Ack to Ack timeout task".into())
            }).map(|_| None)
        } else {
            Err(Error::Internal(
                format!(
                    "InternalError: Ack timeout task channel not found for id: {}",
                    id
                )
                .into(),
            ))
        }
    }

    fn handle_subscription_inner(
        &mut self,
        topic: String,
        item: SubscriptionItem,
    ) -> Result<Option<ClientWriterItem>, Error> {
        if let Some(sub) = self.subscriptions.get(&topic) {
            match sub.try_send(item) {
                Ok(_) => Ok(None),
                Err(err) => match err {
                    flume::TrySendError::Disconnected(_) => {
                        self.subscriptions.remove(&topic);
                        Err(Error::Internal(
                            "Subscription recver is Disconnected".into(),
                        ))
                    }
                    _ => Ok(None),
                },
            }
        } else {
            Err(Error::Internal("Topic is not found locally".into()))
        }
    }

    async fn handle_stopping(&mut self) -> Result<Option<ClientWriterItem>, Error> {
        match self.state {
            ClientBrokerState::Started => self.state = ClientBrokerState::Stopping,
            ClientBrokerState::Stopping => self.state = ClientBrokerState::Stopped,
            _ => {
                return Ok(None)
            }
        }

        Ok(Some(ClientWriterItem::Stopping))
    }
}

#[cfg(any(
    all(feature = "tokio_runtime", not(feature = "async_std_runtime")),
    all(feature = "async_std_runtime", not(feature = "tokio_runtime"))
))]
impl<C: Marshal + Send> ClientBroker<C> {
    async fn handle_publish(
        &mut self,
        topic: String,
        body: Box<OutboundBody>,
    ) -> Result<Option<ClientWriterItem>, Error> {
        let id = self.count.fetch_add(1, Ordering::Relaxed);
        let body = Arc::new(C::marshal(&body)?);
        Ok(Some(ClientWriterItem::Publish(id, topic, body)))
    }

    async fn handle_subscription(
        &mut self,
        id: SeqId,
        topic: String,
        item: Box<InboundBody>,
    ) -> Result<Option<ClientWriterItem>, Error> {
        log::debug!("Handling subscription with AckModeNone");

        let item = SubscriptionItem::new(id, item);
        self.handle_subscription_inner(topic, item)
    }
}


#[async_trait]
impl<C: Marshal + Send> Broker for  ClientBroker<C> {
    type Item = ClientBrokerItem;
    type WriterItem = ClientWriterItem;

    async fn op(
        &mut self,
        item: ClientBrokerItem,
        tx: &Sender<ClientBrokerItem>,
    ) -> Result<Option<ClientWriterItem>, Error> {
        match item {
            ClientBrokerItem::Request {
                id,
                service_method,
                duration,
                body,
                resp_tx,
            } => {
                self.handle_request(id, service_method, duration, body, resp_tx)
                    .await
            }
            ClientBrokerItem::Response { id, result } => self.handle_response(id, result),
            ClientBrokerItem::Cancel(id) => self.handle_cancel(id).await,
            ClientBrokerItem::Publish { topic, body } => {
                self.handle_publish(topic, body).await
            }
            ClientBrokerItem::PublishRetry {
                count,
                id,
                topic,
                body,
            } => {
                self.handle_publish_retry(tx, count, id, topic, body)
                    .await
            }
            ClientBrokerItem::Subscribe { topic, item_sink } => {
                self.handle_subscribe(tx, topic, item_sink).await
            }
            ClientBrokerItem::NewLocalSubscriber {
                topic,
                new_item_sink,
            } => self.handle_new_local_subscriber(topic, new_item_sink),
            ClientBrokerItem::Unsubscribe { topic } => {
                self.handle_unsubscribe(topic).await
            }
            ClientBrokerItem::Subscription { id, topic, item } => {
                self.handle_subscription(id, topic, item).await
            }
            ClientBrokerItem::InboundAck(seq_id) => self.handle_inbound_ack(seq_id.0),
            ClientBrokerItem::Stopping => {
                // Stopping ONLY comes from control
                self.handle_stopping().await
            }
            ClientBrokerItem::Stop => {
                self.state = ClientBrokerState::Stopped;
                Ok(Some(ClientWriterItem::Stop))
            }
        }
    }
}
