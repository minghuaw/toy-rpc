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
        use brw::{Context, Running};
        use futures::{Sink, SinkExt};

        use crate::message::AtomicMessageId;

        use super::{writer::ClientWriterItem};
    }
}

use crate::{
    codec::Marshal,
    error::IoError,
    message::MessageId,
    protocol::{InboundBody, OutboundBody},
    pubsub::SeqId,
    Error,
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
    Stop(Option<std::io::Error>),
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

    // pub ack_mode: PhantomData<AckMode>,
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

    async fn handle_request<'w, W>(
        &'w mut self,
        writer: &'w mut W,
        id: MessageId,
        service_method: String,
        duration: Duration,
        body: Box<OutboundBody>,
        resp_tx: oneshot::Sender<Result<ResponseResult, Error>>,
    ) -> Result<(), Error>
    where
        W: Sink<ClientWriterItem, Error = flume::SendError<ClientWriterItem>> + Send + Unpin,
    {
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
        if let Err(_) = writer.send(item).await {
            return Err(Error::IoError(IoError::new(
                std::io::ErrorKind::Other,
                "Writer is disconnected",
            )));
        }

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
        // request_result.map_err(|err| err.into())
        Ok(())
    }

    fn handle_response(&mut self, id: MessageId, result: ResponseResult) -> Result<(), Error> {
        if let Some(tx) = self.pending.remove(&id) {
            tx.send(Ok(result)).map_err(|_| {
                Error::Internal("InternalError: client failed to send response over channel".into())
            })
        } else {
            Err(Error::Internal(
                format!("InternalError: Response channel not found for id: {}", id).into(),
            ))
        }
    }

    async fn handle_cancel<'w, W>(
        &'w mut self,
        writer: &'w mut W,
        id: MessageId,
    ) -> Result<(), Error>
    where
        W: Sink<ClientWriterItem, Error = flume::SendError<ClientWriterItem>> + Send + Unpin,
    {
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
        writer
            .send(ClientWriterItem::Cancel(id))
            .await
            .map_err(|_| {
                Error::IoError(IoError::new(
                    std::io::ErrorKind::Other,
                    "Writer is disconnected",
                ))
            })
    }

    async fn handle_publish_inner<'w, W>(
        writer: &'w mut W,
        id: MessageId,
        topic: String,
        body: Arc<Vec<u8>>,
    ) -> Result<(), Error>
    where
        W: Sink<ClientWriterItem, Error = flume::SendError<ClientWriterItem>> + Send + Unpin,
    {
        writer
            .send(ClientWriterItem::Publish(id, topic, body))
            .await
            .map_err(|_| {
                Error::IoError(IoError::new(
                    std::io::ErrorKind::Other,
                    "Writer is disconnected",
                ))
            })
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

    async fn handle_publish_retry<'w, W>(
        &mut self,
        writer: &mut W,
        ctx: &'w Arc<Context<ClientBrokerItem>>,
        mut count: u32,
        id: MessageId,
        topic: String,
        body: Arc<Vec<u8>>,
    ) -> Result<(), Error>
    where
        W: Sink<ClientWriterItem, Error = flume::SendError<ClientWriterItem>> + Send + Unpin,
    {
        if count < self.max_num_retries {
            count += 1;
            let result = Self::handle_publish_inner(writer, id, topic.clone(), body.clone()).await;
            let broker = ctx.broker.clone();
            self.spawn_timed_task_waiting_for_ack(
                broker,
                count,
                id,
                topic,
                body,
                self.pub_retry_timeout,
            );
            result
        } else {
            Err(Error::MaxRetriesReached(id))
        }
    }

    async fn handle_subscribe<'w, W>(
        &'w mut self,
        writer: &'w mut W,
        topic: String,
        item_sink: Sender<SubscriptionItem>,
    ) -> Result<(), Error>
    where
        W: Sink<ClientWriterItem, Error = flume::SendError<ClientWriterItem>> + Send + Unpin,
    {
        let id = self.count.fetch_add(1, Ordering::Relaxed);
        // NOTE: Only one local subscriber is allowed
        self.subscriptions.insert(topic.clone(), item_sink);

        writer
            .send(ClientWriterItem::Subscribe(id, topic))
            .await
            .map_err(|_| {
                Error::IoError(IoError::new(
                    std::io::ErrorKind::Other,
                    "Writer is disconnected",
                ))
            })
    }

    fn handle_new_local_subscriber(
        &mut self,
        topic: String,
        new_item_sink: Sender<SubscriptionItem>,
    ) -> Result<(), Error> {
        self.subscriptions.insert(topic, new_item_sink);
        Ok(())
    }

    async fn handle_unsubscribe<'w, W>(
        &'w mut self,
        writer: &'w mut W,
        topic: String,
    ) -> Result<(), Error>
    where
        W: Sink<ClientWriterItem, Error = flume::SendError<ClientWriterItem>> + Send + Unpin,
    {
        let id = self.count.fetch_add(1, Ordering::Relaxed);
        // NOTE: the sender should be dropped on the Client side
        writer
            .send(ClientWriterItem::Unsubscribe(id, topic))
            .await
            .map_err(|_| {
                Error::IoError(IoError::new(
                    std::io::ErrorKind::Other,
                    "Writer is disconnected",
                ))
            })
    }

    fn handle_inbound_ack(&mut self, id: MessageId) -> Result<(), Error> {
        if let Some(tx) = self.pending_acks.remove(&id) {
            tx.send(()).map_err(|_| {
                Error::Internal("InternalError: Failed to send Ack to Ack timeout task".into())
            })
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

    // async fn handle_outbound_ack<'w, W>(
    //     &'w mut self,
    //     writer: &'w mut W,
    //     seq_id: SeqId,
    // ) -> Result<(), Error>
    // where
    //     W: Sink<ClientWriterItem, Error = flume::SendError<ClientWriterItem>> + Send + Unpin,
    // {
    //     writer
    //         .send(ClientWriterItem::Ack(seq_id))
    //         .await
    //         .map_err(|_| {
    //             Error::IoError(IoError::new(
    //                 std::io::ErrorKind::Other,
    //                 "Writer is disconnected",
    //             ))
    //         })
    // }

    fn handle_subscription_inner(
        &mut self,
        topic: String,
        item: SubscriptionItem,
    ) -> Result<(), Error> {
        if let Some(sub) = self.subscriptions.get(&topic) {
            match sub.try_send(item) {
                Ok(_) => Ok(()),
                Err(err) => match err {
                    flume::TrySendError::Disconnected(_) => {
                        self.subscriptions.remove(&topic);
                        Err(Error::Internal(
                            "Subscription recver is Disconnected".into(),
                        ))
                    }
                    _ => Ok(()),
                },
            }
        } else {
            Err(Error::Internal("Topic is not found locally".into()))
        }
    }

    async fn handle_stopping<'w, W>(&'w mut self, writer: &'w mut W) -> Result<(), Error>
    where
        W: Sink<ClientWriterItem, Error = flume::SendError<ClientWriterItem>> + Send + Unpin,
    {
        match self.state {
            ClientBrokerState::Started => self.state = ClientBrokerState::Stopping,
            ClientBrokerState::Stopping => self.state = ClientBrokerState::Stopped,
            _ => {
                return Err(Error::IoError(IoError::new(
                    std::io::ErrorKind::NotConnected,
                    "Connection is already closed",
                )))
            }
        }

        writer.send(ClientWriterItem::Stopping).await.map_err(|_| {
            Error::IoError(IoError::new(
                std::io::ErrorKind::Other,
                "Writer is disconnected",
            ))
        })
    }
}

#[cfg(any(
    all(feature = "tokio_runtime", not(feature = "async_std_runtime")),
    all(feature = "async_std_runtime", not(feature = "tokio_runtime"))
))]
impl<C: Marshal + Send> ClientBroker<C> {
    async fn handle_publish<'w, W>(
        &'w mut self,
        writer: &'w mut W,
        _: &'w Arc<Context<ClientBrokerItem>>,
        topic: String,
        body: Box<OutboundBody>,
    ) -> Result<(), Error>
    where
        W: Sink<ClientWriterItem, Error = flume::SendError<ClientWriterItem>> + Send + Unpin,
    {
        let id = self.count.fetch_add(1, Ordering::Relaxed);
        let body = Arc::new(C::marshal(&body)?);
        Self::handle_publish_inner(writer, id, topic, body).await
    }

    async fn handle_subscription<'w, W>(
        &'w mut self,
        _: &'w mut W,
        id: SeqId,
        topic: String,
        item: Box<InboundBody>,
    ) -> Result<(), Error>
    where
        W: Sink<ClientWriterItem, Error = flume::SendError<ClientWriterItem>> + Send + Unpin,
    {
        log::debug!("Handling subscription with AckModeNone");

        let item = SubscriptionItem::new(id, item);
        self.handle_subscription_inner(topic, item)
        // No Ack will be sent
    }
}

// #[cfg(any(
//     all(feature = "tokio_runtime", not(feature = "async_std_runtime")),
//     all(feature = "async_std_runtime", not(feature = "tokio_runtime"))
// ))]
// impl<C: Marshal + Send> ClientBroker<AckModeAuto, C> {
//     async fn handle_publish<'w, W>(
//         &'w mut self,
//         writer: &'w mut W,
//         ctx: &'w Arc<Context<ClientBrokerItem>>,
//         topic: String,
//         body: Box<OutboundBody>,
//     ) -> Result<(), Error>
//     where
//         W: Sink<ClientWriterItem, Error = flume::SendError<ClientWriterItem>> + Send + Unpin,
//     {
//         let count = 0;
//         let id = self.count.fetch_add(1, Ordering::Relaxed);
//         let body = Arc::new(C::marshal(&body)?);
//         let res = Self::handle_publish_inner(writer, id, topic.clone(), body.clone()).await;
//         let broker = ctx.broker.clone();
//         self.spawn_timed_task_waiting_for_ack(
//             broker,
//             count,
//             id,
//             topic,
//             body,
//             self.pub_retry_timeout,
//         );
//         res
//     }

//     async fn handle_subscription<'w, W>(
//         &'w mut self,
//         writer: &'w mut W,
//         id: SeqId,
//         topic: String,
//         item: Box<InboundBody>,
//     ) -> Result<(), Error>
//     where
//         W: Sink<ClientWriterItem, Error = flume::SendError<ClientWriterItem>> + Send + Unpin,
//     {
//         let item = SubscriptionItem::new(id.clone(), item);
//         self.handle_subscription_inner(topic, item)?;
//         // Automatically send back Ack
//         writer.send(ClientWriterItem::Ack(id)).await.map_err(|_| {
//             Error::IoError(IoError::new(
//                 std::io::ErrorKind::Other,
//                 "Writer is disconnected",
//             ))
//         })
//     }
// }

// #[cfg(any(
//     all(feature = "tokio_runtime", not(feature = "async_std_runtime")),
//     all(feature = "async_std_runtime", not(feature = "tokio_runtime"))
// ))]
// impl<C: Marshal + Send> ClientBroker<AckModeManual, C> {
//     async fn handle_publish<'w, W>(
//         &'w mut self,
//         writer: &'w mut W,
//         ctx: &'w Arc<Context<ClientBrokerItem>>,
//         topic: String,
//         body: Box<OutboundBody>,
//     ) -> Result<(), Error>
//     where
//         W: Sink<ClientWriterItem, Error = flume::SendError<ClientWriterItem>> + Send + Unpin,
//     {
//         let count = 0;
//         let id = self.count.fetch_add(1, Ordering::Relaxed);
//         let body = Arc::new(C::marshal(&body)?);
//         let res = Self::handle_publish_inner(writer, id, topic.clone(), body.clone()).await;
//         let broker = ctx.broker.clone();
//         self.spawn_timed_task_waiting_for_ack(
//             broker,
//             count,
//             id,
//             topic,
//             body,
//             self.pub_retry_timeout,
//         );
//         res
//     }

//     async fn handle_subscription<'w, W>(
//         &'w mut self,
//         _: &'w mut W,
//         id: SeqId,
//         topic: String,
//         item: Box<InboundBody>,
//     ) -> Result<(), Error>
//     where
//         W: Sink<ClientWriterItem, Error = flume::SendError<ClientWriterItem>> + Send + Unpin,
//     {
//         log::debug!("Handling subscription with AckModeManual");

//         let item = SubscriptionItem::new(id, item);
//         self.handle_subscription_inner(topic, item)
//         // The user needs to manually Ack
//     }
// }

// macro_rules! impl_broker_for_ack_modes {
//     ($($ack_mode:ty),*) => {
//         $(
//             #[async_trait::async_trait]
//             impl<C: Marshal + Send> brw::Broker for ClientBroker<$ack_mode, C> {
//                 type Item = ClientBrokerItem;
//                 type WriterItem = ClientWriterItem;
//                 type Ok = ();
//                 type Error = Error;

//                 async fn op<W>(
//                     &mut self,
//                     ctx: &Arc<Context<Self::Item>>,
//                     item: Self::Item,
//                     mut writer: W,
//                 ) -> Running<Result<Self::Ok, Self::Error>, Option<Self::Error>>
//                 where
//                     W: Sink<Self::WriterItem, Error = flume::SendError<Self::WriterItem>> + Send + Unpin,
//                 {
//                     let res = match item {
//                         ClientBrokerItem::Request {
//                             id,
//                             service_method,
//                             duration,
//                             body,
//                             resp_tx,
//                         } => {
//                             self.handle_request(&mut writer, id, service_method, duration, body, resp_tx).await
//                         }
//                         ClientBrokerItem::Response { id, result } => {
//                             self.handle_response(id, result)
//                         },
//                         ClientBrokerItem::Cancel(id) => {
//                             self.handle_cancel(&mut writer, id).await
//                         },
//                         ClientBrokerItem::Publish { topic, body } => {
//                             self.handle_publish(&mut writer, ctx, topic, body).await
//                         },
//                         ClientBrokerItem::PublishRetry { count, id, topic, body } => {
//                             self.handle_publish_retry(&mut writer, ctx, count, id, topic, body).await
//                         },
//                         ClientBrokerItem::Subscribe { topic, item_sink } => {
//                             self.handle_subscribe(&mut writer, topic, item_sink).await
//                         },
//                         ClientBrokerItem::NewLocalSubscriber {
//                             topic,
//                             new_item_sink,
//                         } => {
//                             self.handle_new_local_subscriber(topic, new_item_sink)
//                         },
//                         ClientBrokerItem::Unsubscribe { topic } => {
//                             self.handle_unsubscribe(&mut writer, topic).await
//                         },
//                         ClientBrokerItem::Subscription { id, topic, item } => {
//                             self.handle_subscription(&mut writer, id, topic, item).await
//                         },
//                         ClientBrokerItem::InboundAck(seq_id) => {
//                             self.handle_inbound_ack(seq_id.0)
//                         }
//                         ClientBrokerItem::OutboundAck(seq_id) => {
//                             self.handle_outbound_ack(&mut writer, seq_id).await
//                         },
//                         ClientBrokerItem::Stopping => {
//                             // Stopping ONLY comes from control
//                             self.handle_stopping(&mut writer).await
//                         },
//                         ClientBrokerItem::Stop(io_err) => {
//                             // Stop ONLY comes from reader
//                             match self.state {
//                                 ClientBrokerState::Started => {
//                                     if let Err(_) = self.handle_stopping(&mut writer).await {
//                                         todo!()
//                                     }
//                                 },
//                                 ClientBrokerState::Stopping => { },
//                                 ClientBrokerState::Stopped => {
//                                     return Running::Stop(Some(IoError::new(
//                                         std::io::ErrorKind::Other,
//                                         "Connection is already stopped"
//                                     ).into()))
//                                 }
//                             }

//                             if let Err(err) = writer.send(ClientWriterItem::Stop).await {
//                                 log::debug!("{}", err);
//                             }
//                             self.state = ClientBrokerState::Stopped;
//                             return Running::Stop(io_err.map(Into::into))
//                         }
//                     };

//                     Running::Continue(res)
//                 }
//             }
//         )*
//     };
// }

// impl_broker_for_ack_modes!(AckModeNone, AckModeAuto, AckModeManual);

#[async_trait::async_trait]
impl<C: Marshal + Send> brw::Broker for ClientBroker<C> {
    type Item = ClientBrokerItem;
    type WriterItem = ClientWriterItem;
    type Ok = ();
    type Error = Error;

    async fn op<W>(
        &mut self,
        ctx: &Arc<Context<Self::Item>>,
        item: Self::Item,
        mut writer: W,
    ) -> Running<Result<Self::Ok, Self::Error>, Option<Self::Error>>
    where
        W: Sink<Self::WriterItem, Error = flume::SendError<Self::WriterItem>> + Send + Unpin,
    {
        let res = match item {
            ClientBrokerItem::Request {
                id,
                service_method,
                duration,
                body,
                resp_tx,
            } => {
                self.handle_request(&mut writer, id, service_method, duration, body, resp_tx)
                    .await
            }
            ClientBrokerItem::Response { id, result } => self.handle_response(id, result),
            ClientBrokerItem::Cancel(id) => self.handle_cancel(&mut writer, id).await,
            ClientBrokerItem::Publish { topic, body } => {
                self.handle_publish(&mut writer, ctx, topic, body).await
            }
            ClientBrokerItem::PublishRetry {
                count,
                id,
                topic,
                body,
            } => {
                self.handle_publish_retry(&mut writer, ctx, count, id, topic, body)
                    .await
            }
            ClientBrokerItem::Subscribe { topic, item_sink } => {
                self.handle_subscribe(&mut writer, topic, item_sink).await
            }
            ClientBrokerItem::NewLocalSubscriber {
                topic,
                new_item_sink,
            } => self.handle_new_local_subscriber(topic, new_item_sink),
            ClientBrokerItem::Unsubscribe { topic } => {
                self.handle_unsubscribe(&mut writer, topic).await
            }
            ClientBrokerItem::Subscription { id, topic, item } => {
                self.handle_subscription(&mut writer, id, topic, item).await
            }
            ClientBrokerItem::InboundAck(seq_id) => self.handle_inbound_ack(seq_id.0),
            // ClientBrokerItem::OutboundAck(seq_id) => {
            //     self.handle_outbound_ack(&mut writer, seq_id).await
            // },
            ClientBrokerItem::Stopping => {
                // Stopping ONLY comes from control
                self.handle_stopping(&mut writer).await
            }
            ClientBrokerItem::Stop(io_err) => {
                // Stop ONLY comes from reader
                match self.state {
                    ClientBrokerState::Started => {
                        if let Err(_) = self.handle_stopping(&mut writer).await {
                            todo!()
                        }
                    }
                    ClientBrokerState::Stopping => {}
                    ClientBrokerState::Stopped => {
                        return Running::Stop(Some(
                            IoError::new(
                                std::io::ErrorKind::Other,
                                "Connection is already stopped",
                            )
                            .into(),
                        ))
                    }
                }

                if let Err(err) = writer.send(ClientWriterItem::Stop).await {
                    log::debug!("{}", err);
                }
                self.state = ClientBrokerState::Stopped;
                return Running::Stop(io_err.map(Into::into));
            }
        };

        Running::Continue(res)
    }
}
