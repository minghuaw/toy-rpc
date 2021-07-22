use cfg_if::cfg_if;
use flume::Sender;
use futures::channel::oneshot;
use std::time::Duration;

cfg_if! {
    if #[cfg(any(
        all(feature = "tokio_runtime", not(feature = "async_std_runtime")),
        all(feature = "async_std_runtime", not(feature = "tokio_runtime"))
    ))] {
        use std::{sync::{Arc, atomic::Ordering}, collections::HashMap};
        use brw::{Context, Running};
        use futures::{Sink, SinkExt};

        use crate::message::AtomicMessageId;

        use super::{writer::ClientWriterItem};
    }
}

use crate::{
    message::MessageId,
    protocol::{InboundBody, OutboundBody},
    Error,
};

use super::ResponseResult;

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
        // id: MessageId,
        topic: String,
        body: Box<OutboundBody>,
    },
    Subscribe {
        // id: MessageId,
        topic: String,

        // message is deserialized as it is read on the subscriber
        item_sink: Sender<Box<InboundBody>>,
    },
    NewLocalSubscriber {
        topic: String,
        new_item_sink: Sender<Box<InboundBody>>,
    },
    Unsubscribe {
        // id: MessageId,
        topic: String,
    },
    /// Subscription from the server
    Subscription {
        id: MessageId,
        topic: String,
        item: Box<InboundBody>,
    },
    /// (Manual) Ack for incoming Publish message
    Ack,
    /// Stops the broker
    Stop,
}

#[cfg(all(feature = "async_std_runtime", not(feature = "tokio_runtime")))]
use ::async_std::task::{self};
#[cfg(all(feature = "tokio_runtime", not(feature = "async_std_runtime")))]
use ::tokio::task::{self};

#[cfg(any(
    all(feature = "tokio_runtime", not(feature = "async_std_runtime")),
    all(feature = "async_std_runtime", not(feature = "tokio_runtime"))
))]
pub(crate) struct ClientBroker {
    pub count: Arc<AtomicMessageId>,
    pub pending: HashMap<MessageId, oneshot::Sender<Result<ResponseResult, Error>>>,
    pub next_timeout: Option<Duration>,
    pub subscriptions: HashMap<String, Sender<Box<InboundBody>>>,
}

#[cfg(any(
    all(feature = "tokio_runtime", not(feature = "async_std_runtime")),
    all(feature = "async_std_runtime", not(feature = "tokio_runtime"))
))]
impl ClientBroker {
    async fn handle_request<'w, W>(
        &'w mut self, 
        writer: &'w mut W,
        id: MessageId, 
        service_method: String, 
        duration: Duration, 
        body: Box<OutboundBody>, 
        resp_tx: oneshot::Sender<Result<ResponseResult, Error>>
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
                Err(_) => Err(Error::Canceled(Some(id))),
            }
        };
        let request_result = writer
            .send(ClientWriterItem::Request(
                id,
                service_method,
                duration,
                body,
            ))
            .await;

        task::spawn(async move {
            #[cfg(all(feature = "tokio_runtime", not(feature = "async_std_runtime")))]
            let timout_result = ::tokio::time::timeout(duration, fut).await;
            #[cfg(all(feature = "async_std_runtime", not(feature = "tokio_runtime")))]
            let timout_result = ::async_std::future::timeout(duration, fut).await;

            let cancellation_result = match timout_result {
                Ok(res) => res,
                Err(_) => {
                    if let Err(_) = resp_tx.send(Err(Error::Timeout(Some(id)))) {
                        log::trace!("InternalError: Unable to send Error::Timeout(Some({})) over response channel, response receiver is dropped", id);
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
        request_result.map_err(|err| err.into())
    }

    async fn handle_response(&mut self, id: MessageId, result: ResponseResult) -> Result<(), Error> {
        if let Some(tx) = self.pending.remove(&id) {
            tx.send(Ok(result)).map_err(|_| {
                Error::Internal(
                    "InternalError: client failed to send response over channel".into(),
                )
            })
        } else {
            Err(Error::Internal(
                format!("InternalError: Response channel not found for id: {}", id).into(),
            ))
        }
    }

    async fn handle_cancel<'w, W>(&'w mut self, writer: &'w mut W, id: MessageId) -> Result<(), Error> 
    where 
        W: Sink<ClientWriterItem, Error = flume::SendError<ClientWriterItem>> + Send + Unpin,
    {
        if let Some(tx) = self.pending.remove(&id) {
            tx.send(Err(Error::Canceled(Some(id))))
                .map_err(|_| Error::Internal(
                    format!(
                        "Unable to send Error::Canceled(Some({})) over response channel",
                        id
                    )
                    .into(),
                ))?;
        }
        writer
            .send(ClientWriterItem::Cancel(id))
            .await
            .map_err(|err| err.into())
    }

    async fn handle_publish<'w, W>(&'w mut self, writer: &'w mut W, topic: String, body: Box<OutboundBody>) -> Result<(), Error> 
    where 
        W: Sink<ClientWriterItem, Error = flume::SendError<ClientWriterItem>> + Send + Unpin,
    {
        let id = self.count.fetch_add(1, Ordering::Relaxed);
                // TODO: QoS check? at least once?
        let res = writer
            .send(ClientWriterItem::Publish(id, topic, body))
            .await
            .map_err(|err| err.into());

        // TODO: Spawn a timed task to check Ack?
        res
    }

    async fn handle_subscribe<'w, W>(&'w mut self, writer: &'w mut W, topic: String, item_sink: Sender<Box<InboundBody>>) -> Result<(), Error> 
    where 
        W: Sink<ClientWriterItem, Error = flume::SendError<ClientWriterItem>> + Send + Unpin,
    {
        let id = self.count.fetch_add(1, Ordering::Relaxed);
        // NOTE: Only one local subscriber is allowed
        self.subscriptions.insert(topic.clone(), item_sink);

        let res = writer
            .send(ClientWriterItem::Subscribe(id, topic))
            .await
            .map_err(|err| err.into());
        // TODO: Spawn a timed task to check Ack?
        res
    }

    fn handle_new_local_subscriber(&mut self, topic: String, new_item_sink: Sender<Box<InboundBody>>) -> Result<(), Error> {
        self.subscriptions.insert(topic, new_item_sink);
        Ok(())
    }

    async fn handle_unsubscribe<'w, W>(&'w mut self, writer: &'w mut W, topic: String) -> Result<(), Error> 
    where 
        W: Sink<ClientWriterItem, Error = flume::SendError<ClientWriterItem>> + Send + Unpin,
    {
        let id = self.count.fetch_add(1, Ordering::Relaxed);
        // NOTE: the sender should be dropped on the Client side
        let res = writer
            .send(ClientWriterItem::Unsubscribe(id, topic))
            .await
            .map_err(|err| err.into());
        // TODO: Spawn  timed task to check Ack?
        res
    }

    fn handle_subscription(&mut self, _: MessageId, topic: String, item: Box<InboundBody>) -> Result<(), Error> {
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

    async fn handle_ack(&mut self) -> Result<(), Error> {
        unimplemented!()
    }
}

#[cfg(any(
    all(feature = "tokio_runtime", not(feature = "async_std_runtime")),
    all(feature = "async_std_runtime", not(feature = "tokio_runtime"))
))]
#[async_trait::async_trait]
impl brw::Broker for ClientBroker {
    type Item = ClientBrokerItem;
    type WriterItem = ClientWriterItem;
    type Ok = ();
    type Error = Error;

    async fn op<W>(
        &mut self,
        _: &Arc<Context<Self::Item>>,
        item: Self::Item,
        mut writer: W,
    ) -> Running<Result<Self::Ok, Self::Error>>
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
                self.handle_request(&mut writer, id, service_method, duration, body, resp_tx).await
            }
            ClientBrokerItem::Response { id, result } => {
                self.handle_response(id, result).await
            },
            ClientBrokerItem::Cancel(id) => {
                self.handle_cancel(&mut writer, id).await
            },
            ClientBrokerItem::Publish { topic, body } => {
                self.handle_publish(&mut writer, topic, body).await
            },
            ClientBrokerItem::Subscribe { topic, item_sink } => {
                self.handle_subscribe(&mut writer, topic, item_sink).await
            },
            ClientBrokerItem::NewLocalSubscriber {
                topic,
                new_item_sink,
            } => {
                self.handle_new_local_subscriber(topic, new_item_sink)
            },
            ClientBrokerItem::Unsubscribe { topic } => {
                self.handle_unsubscribe(&mut writer, topic).await
            },
            ClientBrokerItem::Subscription { id, topic, item } => {
                self.handle_subscription(id, topic, item)
            }
            ClientBrokerItem::Ack => {
                self.handle_ack().await
            },
            ClientBrokerItem::Stop => {
                if let Err(err) = writer.send(ClientWriterItem::Stop).await {
                    log::error!("{:?}", err);
                }
                return Running::Stop;
            }
        };

        Running::Continue(res)
    }
}
